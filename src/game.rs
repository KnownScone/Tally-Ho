use component as comp;
use resource as res;
use system as sys;

use std::time::{Instant};

use specs::RunNow;
use specs;

pub struct Game<'a> {
    dt: f32,
    logic_disp: specs::Dispatcher<'static, 'a>,
    render_disp: specs::Dispatcher<'static, 'a>,
    
    accumumlator: f32,
    last_update: Option<Instant>,

    pub world: specs::World,
}

impl<'a> Game<'a> {
    pub fn new(dt: f32, mut logic_disp: specs::Dispatcher<'static, 'a>, mut render_disp: specs::Dispatcher<'static, 'a>) -> Game<'a> {
        let mut world = specs::World::new();
        
        world.add_resource(res::DeltaTime(dt));   
        
        // Register the components and resources used in the registered systems (with default values)
        logic_disp.setup(&mut world.res);
        render_disp.setup(&mut world.res);

        Game {
            dt,
            logic_disp,
            render_disp,
            accumumlator: 0.0,
            last_update: None,
            world,
        }
    }

    pub fn update(&mut self, time_scale: f32) {
        // This update's delta time is the constant delta time multiplied by this update's time scale.
        (*self.world.write_resource::<res::DeltaTime>()).0 = self.dt;

        let frame_time = if let Some(lu) = self.last_update {
            let dur = lu.elapsed();
            dur.as_secs() as f32 + dur.subsec_nanos() as f32 / 1_000_000_000.0
        } else {
            0.0
        };

        self.accumumlator += frame_time * time_scale;
        while self.accumumlator >= self.dt {
            use specs::RunNow;
            self.logic_disp.dispatch(&mut self.world.res);
            self.world.exec(|mut tran: specs::WriteStorage<comp::Transform>| {
                use specs::Join;

                for mut tran in (&mut tran.restrict_mut()).join() {
                    if relative_ne!(tran.get_unchecked().last_pos, tran.get_unchecked().pos) {
                        let tran = tran.get_mut_unchecked();
                        tran.last_pos = tran.pos;
                    }
                }
            });
            self.world.maintain();

            self.accumumlator -= self.dt;
        }
        
        self.render_disp.dispatch(&mut self.world.res);

        self.last_update = Some(Instant::now());
    }
}