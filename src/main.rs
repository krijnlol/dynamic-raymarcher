extern crate sdl2;
extern crate colorgrad;
extern crate vecmath;
extern crate spin_sleep;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::render::TextureQuery;
use sdl2::rect::Rect;

use spin_sleep::LoopHelper;

use vecmath::*;

const SCREEN_WIDTH: u32 = 600;
const SCREEN_HEIGHT: u32 = 400;
const RENDER_RES: f32 = 0.25;

fn modulo(a: f64, b: f64) -> f64 {
    a - (b * (a / b).floor())
}

fn sine_wave(x:f64,min:f64,max:f64,period:f64) -> f64 {
    min+((360_f64.to_radians()/period*x).sin()+1_f64)/2_f64*(max-min)
}

// handle the annoying Rect i32
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

fn sphere_sdf(p:Vector3<f64>,r:f64) -> f64 {
    vec3_len(p) - r
}

fn box_sdf(p:Vector3<f64>,b:Vector3<f64>) -> f64 {
    let q = vec3_sub(p.map(|x| x.abs()),b);
    if q.iter().sum::<f64>() > 0.0 {
        return vec3_len(q) + q[0].max(q[1].max(q[2])).min(0.0);
    }
    vec3_len([0.0,0.0,0.0]) + q[0].max(q[1].max(q[2])).min(0.0)
}

fn union_sdf_op(x: Vec<f64>) -> f64 {
    x.into_iter().reduce(f64::min).unwrap().to_owned()

}

fn subtract_sdf_op(a:f64,b:f64) -> f64 {
    (-a).max(b)
}

fn calc_normal(p:Vector3<f64>,t:f64) -> Vector3<f64>
{
    let d0 = de(p,t);
    let epsilon = 0.0001;
    let d1 = [
        de(vec3_sub(p,[epsilon,0.0,0.0]),t),
        de(vec3_sub(p,[0.0,epsilon,0.0]),t),
        de(vec3_sub(p,[0.0,0.0,epsilon]),t)];
    vec3_normalized(vec3_sub([d0,d0,d0],d1))
}



fn de(p:Vector3<f64>,t:f64) -> f64 {
    let repetative_p = p.map(|x| modulo(x+0.5*5.0,5.0)-0.5*5.0);
    subtract_sdf_op(sphere_sdf(repetative_p, sine_wave(t,2.75,3.0,1000.0)),
                    box_sdf(repetative_p, [5_f64,5_f64,5_f64]))
}

fn ray_march(o:Vector3<f64>,dir:Vector3<f64>,t:f64) -> [u8; 3] {
    let mut d_traversed: f64 = 0.0;
    let mut p = vec3_add(o,vec3_mul(dir,[d_traversed,d_traversed,d_traversed]));
    for _step in 0..100 {
        let d = de(p,t);
        if d < 0.01{
            let prep  = vec3_mul(vec3_mul(vec3_add(calc_normal(p,t),[1.0,1.0,1.0]),[0.5,0.5,0.5]),[255.0,255.0,255.0]);
            return [prep[0] as u8,prep[1] as u8,prep[2] as u8];
        } else if d_traversed > 100.0 {
            break;
        }
        d_traversed += d; p = vec3_add(p,vec3_mul(dir,[d,d,d]));
    }
    [0,0,0]
}

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsys = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let window = video_subsys
        .window(
            "dynamic raymarcher",
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
        )
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let font_path = "fonts/OpenSans-VariableFont_wdth,wght.ttf";
    let mut font = ttf_context.load_font(font_path, 128)?;
    font.set_style(sdl2::ttf::FontStyle::BOLD);

    canvas.set_draw_color(pixels::Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut running:bool = true;

    let mut loop_helper = LoopHelper::builder()
    .report_interval_s(0.5) // report every half a second
    .build_with_target_rate(60.0); // limit to 250 FPS if possible

    let mut current_fps = None;

    let mut cam_pos: Vector3<f64> = [0.0,0.0,-2.0];
    let mut cam_up: Vector3<f64> = [0.0,1.0,0.0];
    let mut cam_forward: Vector3<f64> = [0.0,0.0,1.0];
    let mut cam_rot: Vector3<f64> = [(0.0 as f64).to_radians(),(0.0 as f64).to_radians(),(0.0 as f64).to_radians()];

    let mut events = sdl_context.event_pump()?;
    let mut rel_mouse_state;

    let frame_width: u32 = ((SCREEN_WIDTH as f32)*RENDER_RES) as u32;
    let frame_height: u32 = ((SCREEN_HEIGHT as f32)*RENDER_RES) as u32;

    let mut frame = texture_creator.create_texture_target(None,
    frame_width,
    frame_height).expect("");

    sdl_context.mouse().show_cursor(false);
    sdl_context.mouse().set_relative_mouse_mode(true);

    let mut time = 0;

    while running {

        let delta = loop_helper.loop_start();

        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => running=false,//tx.send((cam_pos_offset,false)),

                Event::KeyDown {keycode: Some(keycode),..} => {
                    let up = cam_up;
                    let right = vec3_cross(cam_forward,cam_up);
                    let speed = delta.as_millis() as f64/50.0;
                    match keycode {
                        Keycode::Escape => running = false,
                        Keycode::W => cam_pos = vec3_add(cam_pos,vec3_mul(cam_forward,[0.1,0.1,0.1].map(|x| speed))),
                        Keycode::S => cam_pos = vec3_sub(cam_pos,vec3_mul(cam_forward,[0.1,0.1,0.1].map(|x| speed))),
                        Keycode::A => cam_pos = vec3_sub(cam_pos,vec3_mul(right,[0.1,0.1,0.1].map(|x| speed))),
                        Keycode::D => cam_pos = vec3_add(cam_pos,vec3_mul(right,[0.1,0.1,0.1].map(|x| speed))),
                        Keycode::Q => cam_pos = vec3_sub(cam_pos,vec3_mul(up,[0.1,0.1,0.1].map(|x| speed))),
                        Keycode::E => cam_pos = vec3_add(cam_pos,vec3_mul(up,[0.1,0.1,0.1].map(|x| speed))),
                        _ => {},
                    }

                },

                _ => {}
            }
        }


        rel_mouse_state = events.relative_mouse_state();
        cam_rot = [modulo(cam_rot[0]-rel_mouse_state.y() as f64/100_f64,360_f64.to_radians()),
                   modulo(cam_rot[1]-rel_mouse_state.x() as f64/100_f64,360_f64.to_radians()),
                   cam_rot[2]];

        canvas.with_texture_canvas(&mut frame, |texture_canvas| {
            for x in 0..frame_width {
                for y in 0..frame_height {
                    let fov_adjustment = ((60.0_f64.to_radians() / 2.0) as f64).tan();
                    let aspect_ratio = (frame_width as f64) / (frame_height as f64);
                    let sensor_x = ((((x as f64 + 0.5) / frame_width as f64) * 2.0 - 1.0) * aspect_ratio) * fov_adjustment;
                    let sensor_y = (1.0 - ((y as f64 + 0.5) / frame_height as f64) * 2.0) * fov_adjustment;

                    let temp_theta_x: f64 = sensor_y.atan2(1_f64);
                    let temp_r_x: f64 = ((sensor_y*sensor_y)+1.0).sqrt();

                    let temp_z = temp_r_x*((temp_theta_x+cam_rot[0]).cos());
                    let temp_y = temp_r_x*((temp_theta_x+cam_rot[0]).sin());

                    let temp_theta_y: f64 = temp_z.atan2(sensor_x);
                    let temp_r_y: f64 = ((sensor_x*sensor_x)+(temp_z*temp_z)).sqrt();

                    if x == frame_width/2 && y == frame_height/2{
                        cam_forward = [-(temp_r_y*((temp_theta_y+cam_rot[1]).cos())),temp_y, temp_r_y*((temp_theta_y+cam_rot[1]).sin())];
                    }

                    let color = ray_march(cam_pos,[-(temp_r_y*((temp_theta_y+cam_rot[1]).cos())),
                                                temp_y,
                                                temp_r_y*((temp_theta_y+cam_rot[1]).sin())]/*[-sensor_x,sensor_y,1.0]*/,time as f64);
                    texture_canvas.pixel(x as i16, y as i16, pixels::Color::RGB(color[0],color[1],color[2])).expect("");
                }
            }
        });

        canvas.copy(&frame, None, rect!(0,0,SCREEN_WIDTH,SCREEN_HEIGHT))?;
        
        if let Some(fps) = loop_helper.report_rate() {
            current_fps = Some(fps);
        }

        if current_fps.is_some() {
            let surface = font
                .render(&format!("fps: {:.2}",current_fps.unwrap()))
                .blended(pixels::Color::RGBA(255, 0, 0, 255))
                .map_err(|e| e.to_string())?;
            let texture = texture_creator
                .create_texture_from_surface(&surface)
                .map_err(|e| e.to_string())?;

            let TextureQuery { width, height, .. } = texture.query();

            let padding = 64;
            let target = rect!(40, 40, 100,40);

            canvas.copy(&texture, None, Some(target))?;
        }
        canvas.present();

        time+=delta.as_millis();
    }
    Ok(())
}
