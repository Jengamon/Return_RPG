// Based (read: ripped) off of resolv by @solarlune

use std::convert::AsRef;
use crate::generation::{GenerationalIndex, GenerationalIndexAllocator, GenerationalIndexArray};
use cgmath::{prelude::*, Vector2, Point2, dot};

#[derive(Clone, Debug)]
pub struct Shape {
    x: f32,
    y: f32,
    tags: Vec<String>,
    collidable: bool,
    stype: ShapeType
}

#[derive(Clone, Debug)]
pub struct Collision {
    // Displacement of shape to point of collision
    pub resolve_x: f32,
    pub resolve_y: f32,
    pub shape_a: ShapeIndex,
    pub shape_b: ShapeIndex,
}

#[derive(Clone, Debug)]
enum ShapeType {
    AABB(AABB),
    Compound(Compound),
}

impl ShapeType {
    pub fn shifted(&self, dx: f32, dy: f32) -> ShapeType {
        match self {
            ShapeType::AABB(ref aabb) => {
                ShapeType::AABB(*aabb)
            },
            ShapeType::Compound(ref compound) => {
                ShapeType::Compound(Compound(compound.0.iter().map(|x| x.shifted(dx, dy)).collect()))
            },
        }
    }
}

#[derive(Clone, Debug)]
struct Compound(Vec<Shape>);

#[derive(Clone, Debug, Copy)]
struct AABB {
    w: f32,
    h: f32
}

pub fn generate_edges(verticies: Vec<Point2<f32>>) -> Vec<Vector2<f32>> {
    let mut edges = vec![];

    for i in 0..verticies.len() {
        let edge = verticies[(i + 1) % verticies.len()] - verticies[i];
        edges.push(edge);
    }

    edges
}

use std::ops::Neg;
pub fn orthogonal<T: Copy + Neg<Output=T>>(v: Vector2<T>) -> Vector2<T> {
    Vector2::new(-v[1], v[0])
}

pub fn is_separating_axis(o: Vector2<f32>, p1: Vec<Point2<f32>>, p2: Vec<Point2<f32>>) -> Option<Vector2<f32>> {
    let (mut min1, mut max1) = (std::f32::MAX, std::f32::MIN);
    let (mut min2, mut max2) = (std::f32::MAX, std::f32::MIN);

    for v in p1.iter() {
        let v = v - Point2::new(0.0, 0.0);
        let proj = dot(v,o);

        min1 = min1.min(proj);
        max1 = max1.max(proj);
    }

    for v in p2.iter() {
        let v = v - Point2::new(0.0, 0.0);
        let proj = dot(v,o);

        min2 = min2.min(proj);
        max2 = max2.max(proj);
    }

    if max1 >= min2 && max2 >= min1 {
        let d = (max2 - min1).min(max1 - min2);
        let d_o2 = d / dot(o, o);// + 1e-10;
        let pv = o * d_o2;
        Some(pv)
    } else {
        None
    }
}

pub fn centers_displacement(p1: Vec<Point2<f32>>, p2: Vec<Point2<f32>>) -> Vector2<f32> {
    use cgmath::EuclideanSpace;

    let c1 = Point2::centroid(&p1);
    let c2 = Point2::centroid(&p2);
    c2 - c1
}

impl AABB {
    fn new(w: f32, h: f32) -> AABB {
        AABB {
            w, h,
        }
    }

    fn generate_points(&self, s: &Shape) -> Vec<Point2<f32>> {
        vec![
            Point2::new(s.x, s.y),
            Point2::new(s.x + self.w, s.y),
            Point2::new(s.x + self.w, s.y + self.h),
            Point2::new(s.x, s.y + self.h)
        ]
    }

    fn generate_edges(&self, s: &Shape) -> Vec<Vector2<f32>> {
        generate_edges(self.generate_points(s))
    }
}

impl Shape {
    pub fn new_rectangle_xywh(x: f32, y: f32, w: f32, h: f32) -> Shape {
        Shape {
            x, y,
            tags: vec![],
            collidable: true,
            stype: ShapeType::AABB(AABB {
                w, h,
            })
        }
    }

    pub fn new_rectangle_xyxy(x1: f32, y1: f32, x2: f32, y2: f32) -> Shape {
        Shape {
            x: x1, y: y1,
            tags: vec![],
            collidable: true,
            stype: ShapeType::AABB(AABB {
                w: x2 - x1,
                h: y2 - y1,
            })
        }
    }

    pub fn new_compound(shapes: Vec<Shape>) -> Shape {
        Shape {
            x: 0.0, y: 0.0,
            tags: vec![],
            collidable: true,
            stype: ShapeType::Compound(Compound(shapes))
        }
    }

    pub fn tags(&self) -> &Vec<String> { &self.tags }
    pub fn tags_mut(&mut self) -> &mut Vec<String> { &mut self.tags }
    pub fn has_tags(&mut self, tags_to_query: &[impl AsRef<str>]) -> bool {
        tags_to_query.iter().all(|x| self.tags.contains(&x.as_ref().to_string()))
    }

    fn aabb_aabb(a: &Shape, a_t: &AABB, b: &Shape, b_t: &AABB) -> Option<Vector2<f32>> {
        let a_edges = a_t.generate_edges(a);
        let b_edges = b_t.generate_edges(b);
        let a_points = a_t.generate_points(a);
        let b_points = b_t.generate_points(b);

        let edges: Vec<_> = a_edges.into_iter().chain(b_edges.into_iter()).collect();

        let mut push_vectors = vec![];
        for o in edges.into_iter().map(|x| orthogonal(x)) {
            let pv = is_separating_axis(o, a_points.clone(), b_points.clone());
            if let Some(pv) = pv {
                push_vectors.push(pv);
            } else {
                return None
            }
        }

        let mut mpv = push_vectors.into_iter().fold(Vector2::new(std::f32::MAX, 0.0), |acc, x| {
            let al = dot(acc, acc);
            let xl = dot(x, x);
            if xl > al { acc } else { x }
        });
        let d = centers_displacement(a_points, b_points);
        if dot(d, mpv) > 0.0 {
            mpv = -mpv;
        }

        // Fix bug where somehow we calculate a zero magnitude collision...or very low magnitude collisions
        if mpv.magnitude() > 0.0 {
            Some(mpv)
        } else {
            None
        }
    }

    fn aabb_compound(a: &Shape, a_t: &AABB, b: &Shape, b_t: &Compound) -> Option<Vector2<f32>> {
        b_t.0.iter().find_map(|x| a.is_colliding(x))
    }

    pub fn is_colliding(&self, o: &Shape) -> Option<Vector2<f32>> {
        // Main colliding logic
        if self.collidable && o.collidable {
            match self.stype {
                ShapeType::AABB(ref aabb) => {
                    match o.stype {
                        ShapeType::AABB(ref aabb2) => {
                            Shape::aabb_aabb(self, aabb, o, aabb2)
                        },
                        ShapeType::Compound(ref compound) => {
                            Shape::aabb_compound(self, aabb, o, compound)
                        },
                    }
                },
                ShapeType::Compound(ref compound) => {
                    None // TEMPORARY Until I'm unlazy
                },
            }
        } else {
            None
        }
    }

    pub fn collidable(&self) -> bool { self.collidable }
    pub fn collidable_mut(&mut self) -> &mut bool { &mut self.collidable }
    pub fn x(&self) -> f32 { self.x }
    pub fn x_mut(&mut self) -> &mut f32 { &mut self.x }
    pub fn y(&self) -> f32 { self.y }
    pub fn y_mut(&mut self) -> &mut f32 { &mut self.y }
    pub fn shift(&mut self, x: f32, y: f32) {
        self.x += x; self.y += y;
    }
    pub fn shifted(&self, dx: f32, dy: f32) -> Shape {
        Shape {
            x: self.x + dx,
            y: self.y + dy,
            tags: self.tags.clone(),
            collidable: self.collidable,
            stype: self.stype.shifted(dx, dy)
        }
    }
}

pub type ShapeIndex = GenerationalIndex;
#[derive(Debug)]
pub struct Space {
    shapes: Vec<GenerationalIndex>,
    shapes_array: GenerationalIndexArray<Shape>,
    shapes_allocator: GenerationalIndexAllocator,
}

impl Default for Space {
    fn default() -> Space {
        Space::new()
    }
}

impl Space {
    pub fn new() -> Space {
        Space {
            shapes: vec![],
            shapes_array: GenerationalIndexArray::new(),
            shapes_allocator: GenerationalIndexAllocator::new(),
        }
    }

    pub fn add_shape(&mut self, shape: Shape) -> ShapeIndex {
        let ni = self.shapes_allocator.allocate();
        self.shapes.push(ni);
        self.shapes_array.set(ni, shape);
        ni
    }

    pub fn remove_shape(&mut self, shape_index: ShapeIndex) {
        self.shapes_allocator.deallocate(shape_index);
        self.shapes.retain(|&x| x != shape_index);
    }

    pub fn shape(&self, shape_index: ShapeIndex) -> Option<&Shape> {
        self.shapes_array.get(shape_index)
    }

    pub fn shape_mut(&mut self, shape_index: ShapeIndex) -> Option<&mut Shape> {
        self.shapes_array.get_mut(shape_index)
    }

    pub fn check_collisions(&mut self, shape_index: ShapeIndex) -> Vec<Collision> {
        if self.shapes_allocator.is_live(shape_index) {
            let shape = self.shapes_array.get(shape_index).unwrap();
            let mut colls = vec![];
            for si in self.shapes.iter().filter(|&x| *x != shape_index) {
                let other = self.shapes_array.get(*si).unwrap();
                if let Some(mpv) = shape.is_colliding(other) {
                    colls.push(Collision {
                        resolve_x: mpv.x,
                        resolve_y: mpv.y,
                        shape_a: shape_index,
                        shape_b: *si,
                    });
                }
            }
            colls
        } else {
            vec![]
        }
    }
}