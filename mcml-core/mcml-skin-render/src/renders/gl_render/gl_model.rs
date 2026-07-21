use glam::{Vec2, Vec3};
use glow::{Context, HasContext, Buffer, VertexArray};

pub struct VaoItem {
    pub vertex_buffer_object: Buffer,
    pub index_buffer_object: Buffer,
    pub vertex_array_object: VertexArray,
}

impl VaoItem {
    pub fn new(gl: &Context) -> Self {
        VaoItem {
            vertex_buffer_object: unsafe { gl.create_buffer().unwrap() },
            index_buffer_object: unsafe { gl.create_buffer().unwrap() },
            vertex_array_object: unsafe { gl.create_vertex_array().unwrap() },
        }
    }

    pub fn delete(&self, gl: &Context) {
        unsafe {
            gl.delete_vertex_array(self.vertex_array_object);
            gl.delete_buffer(self.vertex_buffer_object);
            gl.delete_buffer(self.index_buffer_object);
        };
    }
}

pub struct ModelVao {
    pub head: VaoItem,
    pub body: VaoItem,
    pub left_arm: VaoItem,
    pub right_arm: VaoItem,
    pub left_leg: VaoItem,
    pub right_leg: VaoItem,
    pub cape: VaoItem,
}

impl ModelVao {
    pub fn new(gl: &Context) -> Self {
        ModelVao {
            head: VaoItem::new(gl),
            body: VaoItem::new(gl),
            left_arm: VaoItem::new(gl),
            right_arm: VaoItem::new(gl),
            left_leg: VaoItem::new(gl),
            right_leg: VaoItem::new(gl),
            cape: VaoItem::new(gl),
        }
    }

    pub fn delete(&self, gl: &Context) {
        self.head.delete(gl);
        self.body.delete(gl);
        self.left_arm.delete(gl);
        self.right_arm.delete(gl);
        self.left_leg.delete(gl);
        self.right_leg.delete(gl);
        self.cape.delete(gl);
    }
}

#[repr(C)]
pub struct VertexOpenGL {
    pub pos: Vec3,
    pub uv: Vec2,
    pub normal: Vec3
}
