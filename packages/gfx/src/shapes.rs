
/// A compound shape composed of curves and lines that can be drawn by a
/// [`Canvas`](crate::Canvas).
#[derive(Default)]
pub struct Path {
    start_x: f32,
    start_y: f32,
    values: Vec<Value>,
}

impl Path {
    pub fn close(&mut self) {
        self.values.push(Value {
            op: Operation::Close,
        });
        self.start_x = 0.0;
        self.start_y = 0.0;
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        self.values.push(Value {
            op: Operation::Move,
        });
        self.values.push(Value { value: x });
        self.values.push(Value { value: y });
        self.start_x = x;
        self.start_y = y;
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        self.values.push(Value {
            op: Operation::Line,
        });
        self.values.push(Value { value: x });
        self.values.push(Value { value: y });
    }

    pub fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) {
        self.values.push(Value {
            op: Operation::CubicBezier,
        });
        self.values.push(Value { value: x1 });
        self.values.push(Value { value: y1 });
        self.values.push(Value { value: x2 });
        self.values.push(Value { value: y2 });
        self.values.push(Value { value: x3 });
        self.values.push(Value { value: y3 });
    }
}

union Value {
    op: Operation,
    value: f32,
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum Operation {
    Close,
    Move,
    Line,
    CubicBezier,
}
