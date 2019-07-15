use std::collections::HashMap;

use nalgebra as na;

use glutin::{VirtualKeyCode, WindowEvent};

use crate::util::intersection::{ray_aabb_intersection, Ray, AABB};
use crate::machine::grid;
use crate::machine::{Block, PlacedBlock, Machine};
use crate::render::{self, Object, InstanceParams, Resources, Camera, RenderList};

use crate::edit::Edit;

#[derive(Debug, Clone)]
pub struct Config {
    pub rotate_block_key: VirtualKeyCode,

    pub block_keys: HashMap<VirtualKeyCode, Block>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            rotate_block_key: VirtualKeyCode::R,
            block_keys:
                vec![
                    (VirtualKeyCode::Key1, Block::PipeXY),
                    (VirtualKeyCode::Key2, Block::PipeSplitXY),
                    (VirtualKeyCode::Key3, Block::Solid),
                ].into_iter().collect(),
        }
    }
}

pub struct Editor {
    config: Config,

    machine: Machine,

    place_block: PlacedBlock,

    mouse_window_pos: na::Point2<f32>,
    mouse_grid_pos: Option<grid::Point3>,

    render_list: RenderList,
    render_list_transparent: RenderList,
}

impl Editor {
    pub fn new(config: Config, size: grid::Vector3) -> Editor {
        Editor {
            config,
            machine: Machine::new(size),
            place_block: PlacedBlock {
                dir_xy: grid::Dir2(grid::Axis2::X, grid::Sign::Pos),
                block: Block::PipeXY,
            },
            mouse_window_pos: na::Point2::origin(),
            mouse_grid_pos: None,
            render_list: RenderList::new(),
            render_list_transparent: RenderList::new(),
        }
    }

    pub fn run_edit(&mut self, edit: Edit) {
        edit.run(&mut self.machine);
    }

    pub fn update(&mut self, dt_secs: f32, camera: &Camera) {
        let p = self.mouse_window_pos;
        let p_near = camera.unproject(&na::Point3::new(p.x, p.y, -1.0));
        let p_far = camera.unproject(&na::Point3::new(p.x, p.y, 1.0));

        let ray = Ray {
            origin: camera.eye(),
            velocity: p_far - p_near,
        };
        let aabb = AABB {
            min: na::Point3::origin(),
            max: na::Point3::new(self.machine.size().x as f32, self.machine.size().y as f32, 1.0),
        };

        let intersection = ray_aabb_intersection(&ray, &aabb);
        self.mouse_grid_pos =
            if let Some(ray_t) = intersection {
                let ray_pos = ray.origin + ray_t * ray.velocity;
                let grid_pos = grid::Point3::new(
                    ray_pos.x.floor() as isize,
                    ray_pos.y.floor() as isize,
                    0, // TODO
                );

                if self.machine.is_valid_pos(&grid_pos) {
                    Some(grid_pos)
                } else {
                    None
                }
            } else {
                None
            }
    }

    pub fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved {
                device_id: _,
                position,
                modifiers: _,
            } => {
                self.mouse_window_pos = na::Point2::new(
                    position.x as f32,
                    position.y as f32,
                );
            }
            WindowEvent::KeyboardInput { device_id: _, input } =>
                self.on_keyboard_input(*input),
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
                modifiers,
            } => self.on_mouse_input(*state, *button, *modifiers),

            _ => ()
        }
    }

    fn on_keyboard_input(&mut self, input: glutin::KeyboardInput) {
        if input.state == glutin::ElementState::Pressed {
            if let Some(keycode) = input.virtual_keycode {
                self.on_key_press(keycode);
            }
        }
    }

    fn on_key_press(&mut self, keycode: VirtualKeyCode) {
        if keycode == self.config.rotate_block_key {
            self.place_block.dir_xy = self.place_block.dir_xy.rotated_cw();
        }

        if let Some(block) = self.config.block_keys.get(&keycode) {
            self.place_block.block = block.clone();
        }
    }

    fn on_mouse_input(
        &mut self,
        state: glutin::ElementState,
        button: glutin::MouseButton,
        _modifiers: glutin::ModifiersState,
    ) {
        if state == glutin::ElementState::Pressed {
            match button {
                glutin::MouseButton::Left => {
                    if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                        let edit = Edit::SetBlock(
                            mouse_grid_pos,
                            Some(self.place_block.clone())
                        );
                        self.run_edit(edit);
                    }
                }
                glutin::MouseButton::Right => {
                    if let Some(mouse_grid_pos) = self.mouse_grid_pos {
                        let edit = Edit::SetBlock(
                            mouse_grid_pos,
                            None
                        );
                        self.run_edit(edit);
                    }
                }
                _ => (),
            }
        }
    }

    pub fn render<S: glium::Surface>(
        &mut self,
        resources: &Resources,
        render_context: &render::Context,
        target: &mut S,
    ) -> Result<(), glium::DrawError> {
        self.render_list.clear();
        self.render_list_transparent.clear();

        render::machine::render_machine(&self.machine, &mut self.render_list);
        render::machine::render_xy_grid(&self.machine.size(), 0.0, &mut self.render_list);

        if let Some(mouse_grid_pos) = self.mouse_grid_pos {
            assert!(self.machine.is_valid_pos(&mouse_grid_pos));

            let mouse_grid_pos_float: na::Point3<f32> = na::convert(mouse_grid_pos);

            render::machine::render_cuboid_wireframe(
                &render::machine::Cuboid {
                    center: mouse_grid_pos_float + na::Vector3::new(0.5, 0.5, 0.51),
                    size: na::Vector3::new(1.0, 1.0, 1.0),
                },
                0.015,
                &na::Vector4::new(0.9, 0.9, 0.9, 1.0),
                &mut self.render_list,
            );

            let block_transform = render::machine::placed_block_transform(
                &mouse_grid_pos,
                &self.place_block.dir_xy,
            );
            render::machine::render_block(
                &self.place_block.block, 
                &block_transform,
                Some(&na::Vector4::new(0.3, 0.5, 0.9, 0.7)),
                &mut self.render_list_transparent,
            );
        }
        
        self.render_list.render(
            &resources,
            &render_context,
            &Default::default(),
            target,
        )?;

        self.render_list_transparent.render(
            &resources,
            &render_context,
            &glium::DrawParameters {
                blend: glium::draw_parameters::Blend::alpha_blending(), 
                .. Default::default()
            },
            target,
        )?;
        
        Ok(())
    }
}
