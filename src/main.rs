mod timer;
mod gamepad;
mod generation;
mod collision;
mod script;

use raylib::prelude::*;
use cgmath::prelude::*;
use cgmath::{Vector2, dot};
use std::time::Duration;
use crate::timer::Timer;
use crate::gamepad::VirtualGamepadState;
use specs::{prelude::*, Component, World, Builder};
use specs::{Read, Write, WriteStorage, ReadStorage, System, Entities};
use std::ops::Deref;
use crate::collision::{Space, ShapeIndex, Shape};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, Default)]
struct DeltaTime(Duration);

#[derive(Debug, Default)]
struct PhysicsSpace(Space);

struct PhysicsSystem {
    shape_index_mapping: HashMap<ShapeIndex, Entity>,
}

impl PhysicsSystem {
    pub fn new() -> PhysicsSystem {
        PhysicsSystem {
            shape_index_mapping: HashMap::new(),
        }
    }
}

impl<'a> System<'a> for PhysicsSystem {
    type SystemData = (Entities<'a>,
                       Read<'a, DeltaTime>,
                       Write<'a, PhysicsSpace>,
                       WriteStorage<'a, Position>,
                       WriteStorage<'a, Velocity>,
                       ReadStorage<'a, Friction>,
                       WriteStorage<'a, CollisionAabb>);

    fn run(&mut self, data: Self::SystemData) {
        let (entities,
            delta,
            mut space,
            mut poss,
            mut vels,
            fric,
            mut aabbs) = data;

        let mut space = &mut space.0;

        // Update shapes and create them if they don't exist.
        for (ent, pos, aabb) in (&entities, &mut poss, &mut aabbs).join() {
            if aabb.shape_index.is_none() {
                // Allocate a shape
                let si = space.add_shape(Shape::new_rectangle_xywh(pos.position.x, pos.position.y, aabb.size.0, aabb.size.1));
                aabb.shape_index = Some(si);
                // Insert the shape into our mapping
                self.shape_index_mapping.insert(si, ent);
            } else {
                // Sync our position with the position of our shape.
                let shape = space.shape_mut(aabb.shape_index.unwrap());
                if let Some(mut shape) = shape {
//                    *shape.x_mut() = pos.position.x;
//                    *shape.y_mut() = pos.position.y;
                    pos.position.x = shape.x();
                    pos.position.y = shape.y();
                }
            }
        }

        let mut test_collisions = |ent: Entity, pos: &mut Position, space: &mut Space| -> Option<Vector2<f32>> {
            let aabb = aabbs.get(ent);
            let mut normal = Vector2::new(0.0, 0.0);

            if let Some(mut aabb) = aabb {
                let si = aabb.shape_index.unwrap(); // This shouldn't fail, cuz we just allocated the shape.
                for coll in space.check_collisions(si) {
                    let oe = self.shape_index_mapping.get(&coll.shape_b).unwrap();
                    let mpv = Vector2::new(coll.resolve_x, coll.resolve_y);
//                    if mpv.magnitude() > 1.0 {
//                        println!("{:?}", mpv);
//                    }
                    pos.position += mpv;
                    let shape = space.shape_mut(si).unwrap();
                    *shape.x_mut() = pos.position.x;
                    *shape.y_mut() = pos.position.y;
                    normal += mpv.normalize();
                    //println!("{:?}", coll);
                    // If we wanted too support pushing, how...
                }
            }

            if normal.magnitude() > 0.0 {
                Some(normal)
            } else {
                None
            }
        };

        let move_ent = |ent: Entity, pos: &mut Position, by: Vector2<f32>, aabb: &CollisionAabb, space: &mut Space| {
            let sweep_steps = 5;
            let mut last_worked = Vector2::new(0.0, 0.0);
            for i in 1..=sweep_steps {
                let percent = i as f32 / sweep_steps as f32;
                let percent_by = by * percent;
                pos.position -= last_worked;
                pos.position += percent_by;
                if let Some(mpv) = test_collisions(ent, pos, space) {
                    return Some(mpv)
                } else {
                    last_worked = percent_by;
                }
            }
            // Update shape by full movement vector
            let shape = space.shape_mut(aabb.shape_index.unwrap()).unwrap();
            *shape.x_mut() = pos.position.x;
            *shape.y_mut() = pos.position.y;
            None
        };

        let dt = delta.0.as_secs_f32();

        for (ent, pos, vel) in (&entities, &mut poss, &mut vels).join() {
            let aabb = aabbs.get(ent);
            if let Some(aabb) = aabb {
                if let Some(mpv) = move_ent(ent, pos, vel.velocity * dt, aabb, &mut space) {
                    // TODO Preserve velocity length.
                    let normal = mpv.normalize();
                    let undesired = normal * dot(vel.velocity, normal);
                    vel.velocity -= undesired;
                }
            } else {
                let vely = vel.velocity * dt;
                pos.position += vely;
            }
        }

        drop(move_ent);
        drop(test_collisions);

        for(ent, vel) in (&entities, &mut vels).join() {
            let fric: Option<&Friction> = fric.get(ent);
            if let Some(fric) = fric {
                vel.velocity *= (1.0 - fric.friction);
            }
        }
    }
}

struct InputSystem;

impl<'a> System<'a> for InputSystem {
    type SystemData = (Read<'a, VirtualGamepadState>,
                       ReadStorage<'a, ControllerInput>,
                       WriteStorage<'a, Velocity>);

    fn run(&mut self, data: Self::SystemData) {
        let (controller, inputs, mut vels) = data;

//        println!("{:?}", controller.deref());
        let speed = if controller.l_bumper {40.0} else {10.5};
        for(input, vel) in (&inputs, &mut vels).join() {
            let move_vector = Vector2::new(controller.l_x_axis, controller.l_y_axis);
            if move_vector.magnitude() > 0.0 {
                let move_vector = move_vector.normalize();
                vel.velocity += move_vector * speed;
                vel.velocity.x = vel.velocity.x.min(vel.max_velocity.x);
                vel.velocity.y = vel.velocity.y.min(vel.max_velocity.y);
            }
        }
    }
}

struct EventAreaSystem;

// TODO Implement events, which check for collision, then execute a series of actions, based off of
// an event script
impl<'a> System<'a> for EventAreaSystem {
    type SystemData = (Entities<'a>, ReadStorage<'a, Position>);

    fn run(&mut self, data: Self::SystemData) {

    }
}

#[derive(Component, Clone, Debug)]
struct Friction {
    friction: f32, // Inverse 1.0 = all friction (no preserved velocity), 0.0 = no friction
}

#[derive(Component, Clone, Debug)]
struct Position {
    position: Vector2<f32>,
}

#[derive(Component, Clone, Debug)]
struct Velocity {
    velocity: Vector2<f32>,
    max_velocity: Vector2<f32>,
}

#[derive(Component, Clone, Debug)]
struct ControllerInput;

#[derive(Component, Clone, Debug)]
struct Display(DisplayType);

#[derive(Clone, Debug)]
enum DisplayType {
    Rectangle(u32, u32, Color),
}

#[derive(Component, Clone, Debug)]
struct CollisionAabb {
    size: (f32, f32),
    shape_index: Option<ShapeIndex>,
}

// TODO Maybe support arbitrary keybindings [NOTE: Advanced]
fn update_gamepad(rl: &RaylibHandle, cgp: &mut VirtualGamepadState) {
    cgp.l_x_axis = if rl.is_key_down(KeyboardKey::KEY_RIGHT) {
        1.0
    } else if rl.is_key_down(KeyboardKey::KEY_LEFT) {
        -1.0
    } else {
        0.0
    };

    cgp.l_y_axis = if rl.is_key_down(KeyboardKey::KEY_DOWN) {
        1.0
    } else if rl.is_key_down(KeyboardKey::KEY_UP) {
        -1.0
    } else {
        0.0
    };

    // R axis and triggers not mapped for now

    cgp.a_button = rl.is_key_down(KeyboardKey::KEY_Z);
    cgp.b_button = rl.is_key_down(KeyboardKey::KEY_X);
    cgp.x_button = rl.is_key_down(KeyboardKey::KEY_A);
    cgp.y_button = rl.is_key_down(KeyboardKey::KEY_S);
    cgp.select_button = rl.is_key_down(KeyboardKey::KEY_SPACE);
    cgp.start_button = rl.is_key_down(KeyboardKey::KEY_ENTER);
    cgp.l_bumper = rl.is_key_down(KeyboardKey::KEY_Q);
    cgp.r_bumper = rl.is_key_down(KeyboardKey::KEY_W)
}

fn load_image<S: AsRef<Path>>(s: S) -> Result<Image, String> {
    image::open(s.as_ref()).map_err(|e|format!("Error loading image file: {:?}", e)).and_then(|x| {
        let rgba = x.to_rgba();
        let (width, height) = rgba.dimensions();
        let raw_buffer = rgba.into_raw();
        Image::load_image_pro(&raw_buffer, width as i32, height as i32, PixelFormat::UNCOMPRESSED_R8G8B8A8)
    })
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(640, 480)
        .title("RETURN - AN RPG")
        .build();

    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<ControllerInput>();
    world.register::<Display>();
    world.register::<Friction>();
    world.register::<CollisionAabb>();

    let player = world.create_entity()
        .with(Position { position: Vector2::new(0.0, 0.0)})
        .with(Velocity { velocity: Vector2::new(0.0, 0.0), max_velocity: Vector2::new(320.0, 320.0)})
        .with(ControllerInput)
        .with(Display(DisplayType::Rectangle(32, 32, Color::RAYWHITE)))
        .with(Friction{ friction: 0.05 })
        .with(CollisionAabb {size:(32.0, 32.0), shape_index: None})
        .build();

    world.create_entity()
        .with(Position{ position: Vector2::new(32.0, 32.0)})
        .with(Display(DisplayType::Rectangle(128, 32, Color::RED)))
        .with(CollisionAabb {size:(128.0, 32.0), shape_index: None})
        .build();

    world.create_entity()
        .with(Position{ position: Vector2::new(96.0, 64.0)})
        .with(Display(DisplayType::Rectangle(32, 128, Color::RED)))
        .with(CollisionAabb {size:(32.0, 128.0), shape_index: None})
        .build();

    let mut timer = Timer::new();

//    let image_load = load_image("test_image.png").unwrap();

    let mut dispatcher = DispatcherBuilder::new()
        .with(InputSystem, "control", &[])
        .with(PhysicsSystem::new(), "physics", &["control"])
        .build();

    let mut current_gamepad = VirtualGamepadState::new();

    world.insert(PhysicsSpace(Space::new()));

//    let texture = rl.load_texture_from_image(&thread, &image_load).unwrap();

    while !rl.window_should_close() {
        // Update gamepad
        update_gamepad(&rl, &mut current_gamepad);

        world.insert(DeltaTime(timer.frame()));
        world.insert(current_gamepad);

        dispatcher.dispatch(&world);

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::new(40, 20, 30 ,255));
        d.draw_text("Hello, world!", 12, 12, 20, Color::RAYWHITE);

        {
            // Draw everything!
            let displays = world.read_storage::<Display>();
            let poss = world.read_storage::<Position>();
            for (pos, disp) in (&poss, &displays).join() {
                match disp.0 {
                    DisplayType::Rectangle(w, h, c) => {
                        d.draw_rectangle(pos.position.x as i32, pos.position.y as i32, w as i32, h as i32, c);
                    }
                }
            }
//            d.draw_texture_ex(&texture, raylib::math::Vector2::new(0.0, 0.0), 0.0, 1.0, Color::WHITE);
        }

        std::thread::sleep(Duration::from_secs_f32(0.016));
    }
}
