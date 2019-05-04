use crate::collision::CollisionDetection;
use amethyst::{
    controls::{CursorHideSystem, HideCursor, MouseFocusUpdateSystem, WindowFocus},
    core::{
        bundle::{Result, SystemBundle},
        nalgebra::{Point3, Unit, Vector3},
        shrev::{EventChannel, ReaderId},
        specs::{Component, DispatcherBuilder, Join, NullStorage, Resources},
        Time, Transform,
    },
    ecs::{Entity, Read, ReadExpect, ReadStorage, System, WriteStorage},
    input::{get_input_axis_simple, InputHandler},
    ui::{UiFinder, UiText},
    winit::{DeviceEvent, Event},
};
use ncollide3d::math::{Point, Vector};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{hash::Hash, marker::PhantomData, sync::Arc};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct PlayerControlTag;
impl Component for PlayerControlTag {
    type Storage = NullStorage<PlayerControlTag>;
}

struct PlayerMovementSystem<A, B> {
    speed: f32,
    // The name of the input axis to locally move in the x coordinates.
    right_input_axis: Option<A>,
    // The name of the input axis to locally move in the y coordinates.
    up_input_axis: Option<A>,
    // The name of the input axis to locally move in the z coordinates.
    forward_input_axis: Option<A>,
    _marker: PhantomData<B>,
}

impl<A, B> PlayerMovementSystem<A, B>
where
    A: Send + Sync + Hash + Eq + Clone + 'static,
    B: Send + Sync + Hash + Eq + Clone + 'static,
{
    pub fn new(
        speed: f32,
        right_input_axis: Option<A>,
        up_input_axis: Option<A>,
        forward_input_axis: Option<A>,
    ) -> Self {
        PlayerMovementSystem {
            speed,
            right_input_axis,
            up_input_axis,
            forward_input_axis,
            _marker: PhantomData,
        }
    }
}

impl<'a, A, B> System<'a> for PlayerMovementSystem<A, B>
where
    A: Send + Sync + Hash + Eq + Clone + 'static,
    B: Send + Sync + Hash + Eq + Clone + 'static,
{
    type SystemData = (
        Read<'a, Time>,
        ReadExpect<'a, Arc<Mutex<CollisionDetection>>>,
        WriteStorage<'a, Transform>,
<<<<<<< HEAD
        //WriteStorage<'a, RayComponent<f32>>,
=======
>>>>>>> master
        Read<'a, InputHandler<A, B>>,
        ReadStorage<'a, PlayerControlTag>,
    );

<<<<<<< HEAD
    fn run(&mut self, (time, mut transform, /*mut ray,*/ input, tag): Self::SystemData) {
=======
    fn run(&mut self, (time, collision, mut transform, input, tag): Self::SystemData) {
>>>>>>> master
        let x = get_input_axis_simple(&self.right_input_axis, &input);
        let y = get_input_axis_simple(&self.up_input_axis, &input);
        let z = get_input_axis_simple(&self.forward_input_axis, &input);

        if let Some(direction) = Unit::try_new(Vector3::new(x, y, z), 1.0e-6) {
<<<<<<< HEAD
            for (transform, /*ray,*/ _) in (&mut transform, /*&mut ray,*/ &tag).join() {
                let translation = transform.translation();
                let origin = Point::new(translation.x, translation.y, translation.z);
                //let dir = ncollide3d::math::Vector::new(direction.x, direction.y, direction.z);
                //*ray = RayComponent::new(Ray::new(origin, dir));
=======
            for (transform, _) in (&mut transform, &tag).join() {
>>>>>>> master
                transform.move_along_local(direction, time.delta_seconds() * self.speed);
                collision
                    .lock()
                    .set_player_pos(Point3::from(*transform.translation()));
            }
        }
    }
}

struct PlayerRotationSystem<A, B> {
    sensitivity_x: f32,
    sensitivity_y: f32,
    _marker1: PhantomData<A>,
    _marker2: PhantomData<B>,
    event_reader: Option<ReaderId<Event>>,
}

impl<A, B> PlayerRotationSystem<A, B> {
    pub fn new(sensitivity_x: f32, sensitivity_y: f32) -> Self {
        PlayerRotationSystem {
            sensitivity_x,
            sensitivity_y,
            _marker1: PhantomData,
            _marker2: PhantomData,
            event_reader: None,
        }
    }
}

impl<'a, A, B> System<'a> for PlayerRotationSystem<A, B>
where
    A: Send + Sync + Hash + Eq + Clone + 'static,
    B: Send + Sync + Hash + Eq + Clone + 'static,
{
    type SystemData = (
        Read<'a, EventChannel<Event>>,
        WriteStorage<'a, Transform>,
        ReadStorage<'a, PlayerControlTag>,
        Read<'a, WindowFocus>,
        Read<'a, HideCursor>,
    );

    fn run(&mut self, (events, mut transform, tag, focus, hide): Self::SystemData) {
        let focused = focus.is_focused;
        for event in events.read(&mut self.event_reader.as_mut().expect(
            "`PlayerRotationSystem::setup` was not called before `PlayerRotationSystem::run`",
        )) {
            if focused && hide.hide {
                if let Event::DeviceEvent { ref event, .. } = *event {
                    if let DeviceEvent::MouseMotion { delta: (x, y) } = *event {
                        for (transform, _) in (&mut transform, &tag).join() {
                            transform.pitch_local((-y as f32 * self.sensitivity_y).to_radians());
                            transform.yaw_global((-x as f32 * self.sensitivity_x).to_radians());
                        }
                    }
                }
            }
        }
    }

    fn setup(&mut self, res: &mut Resources) {
        use amethyst::core::specs::SystemData;

        Self::SystemData::setup(res);
        self.event_reader = Some(res.fetch_mut::<EventChannel<Event>>().register_reader());
    }
}

struct DrawPlayerPositionSystem {
    position_text: Option<Entity>,
}
impl DrawPlayerPositionSystem {
    fn new() -> Self {
        DrawPlayerPositionSystem {
            position_text: None,
        }
    }
}

impl<'a> System<'a> for DrawPlayerPositionSystem {
    type SystemData = (
        ReadStorage<'a, PlayerControlTag>,
        ReadStorage<'a, Transform>,
        WriteStorage<'a, UiText>,
        UiFinder<'a>,
        Read<'a, Time>,
    );

    fn run(&mut self, (player_tag, transforms, mut ui_text, ui_finder, time): Self::SystemData) {
        if self.position_text.is_none() {
            if let Some(entity) = ui_finder.find("position_text") {
                self.position_text = Some(entity);
            }
        }
        for (transform, _) in (&transforms, &player_tag).join() {
            if let Some(position_display) = self.position_text.and_then(|e| ui_text.get_mut(e)) {
                if time.frame_number() % 20 == 0 {
                    let t = transform.translation();
                    let string = format!("{:.0}, {:.0}, {:.0}", t.x, t.y, t.z);
                    position_display.text = string;
                }
            }
        }
    }
}

pub struct PlayerControlBundle<A, B> {
    sensitivity_x: f32,
    sensitivity_y: f32,
    speed: f32,
    right_input_axis: Option<A>,
    up_input_axis: Option<A>,
    forward_input_axis: Option<A>,
    _marker: PhantomData<B>,
}

impl<A, B> PlayerControlBundle<A, B> {
    pub fn new(
        right_input_axis: Option<A>,
        up_input_axis: Option<A>,
        forward_input_axis: Option<A>,
    ) -> Self {
        PlayerControlBundle {
            sensitivity_x: 1.0,
            sensitivity_y: 1.0,
            speed: 1.0,
            right_input_axis,
            up_input_axis,
            forward_input_axis,
            _marker: PhantomData,
        }
    }

    pub fn with_sensitivity(mut self, x: f32, y: f32) -> Self {
        self.sensitivity_x = x;
        self.sensitivity_y = y;
        self
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
}

impl<'a, 'b, A, B> SystemBundle<'a, 'b> for PlayerControlBundle<A, B>
where
    A: Send + Sync + Hash + Eq + Clone + 'static,
    B: Send + Sync + Hash + Eq + Clone + 'static,
{
    fn build(self, builder: &mut DispatcherBuilder<'a, 'b>) -> Result<()> {
        builder.add(
            PlayerMovementSystem::<A, B>::new(
                self.speed,
                self.right_input_axis,
                self.up_input_axis,
                self.forward_input_axis,
            ),
            "player_movement",
            &[],
        );
        builder.add(
            PlayerRotationSystem::<A, B>::new(self.sensitivity_x, self.sensitivity_y),
            "player_rotation",
            &[],
        );
        builder.add(
            MouseFocusUpdateSystem::new(),
            "mouse_focus",
            &["player_rotation"],
        );
        builder.add(CursorHideSystem::new(), "cursor_hide", &["mouse_focus"]);
        builder.add(DrawPlayerPositionSystem::new(), "draw_player_position", &[]);
        Ok(())
    }
}
