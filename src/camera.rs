
use dacite::core::Extent2D;
use siege_math::{Angle, Point3, Mat3, Mat4, Vec4, NQuat, Y_AXIS_F32, X_AXIS_F32};
use std::f32::consts::PI;
use siege_plugin_avatar_simple::Placement;
use Config;

// Field of view depends how far back the viewers head is from the monitor, and
// how wide the monitor is. VR equipment wants a wide one. Normal monitors would
// use a more narrow one. The user should be able to adjust this.
// For a desktop monitor, the FOV should be around 60 degrees.
// Oculus rift goes to 110, targetting 140 for upcoming models.
const MAX_FOV: f32 = 110.0 * PI / 180.0;    // 110 degrees
const DEFAULT_FOV: f32 = 60.0 * PI / 180.0; // 60 degrees
const MIN_FOV: f32 = 45.0 * PI / 180.0;     // 45 degrees

// we measure in meters, so this goes from 25 centimeters to the far plane.
pub const NEAR_PLANE: f32 = 0.25;

// 100 kilometers
pub const FAR_PLANE: f32 = 100_000.0;

/// This represents the camera.
/// Currently, the camera placement is the same as that of the avatar (1st person only)
#[derive(Debug)]
pub struct Camera {
    /// Field of view (Angle for zoom/wide, based on width)
    pub fovx: Angle<f32>,
    /// Window Extent
    pub extent: Extent2D,
    /// View matrix
    pub view_matrix: Mat4<f32>,
    /// Camera model matrix
    pub camera_model_matrix: Mat4<f32>,
    /// Frustum planes (for view frustum culling)
    pub frustum_planes: [Vec4<f32>; 6]
}

impl Camera {
    pub fn new(config: &Config) -> Camera
    {
        let view_matrix = fps_view(&Placement::new(
            Point3::new(0.0, 0.0, 0.0), 0.0, 0.0)); // This gets quickly updated
        let camera_model_matrix = view_matrix;

        // This is bogus data, but it gets quickly updated
        Camera {
            fovx: Angle::<f32>::from_radians(DEFAULT_FOV),
            extent: Extent2D {
                width: config.window.width,
                height: config.window.height,
            },
            view_matrix: view_matrix,
            camera_model_matrix: camera_model_matrix,
            frustum_planes: [Vec4::<f32>::new(0.0, 0.0, 1.0, 0.0),
                             Vec4::<f32>::new(0.0, 0.0, 1.0, 0.0),
                             Vec4::<f32>::new(0.0, 0.0, 1.0, 0.0),
                             Vec4::<f32>::new(0.0, 0.0, 1.0, 0.0),
                             Vec4::<f32>::new(0.0, 0.0, 1.0, 0.0),
                             Vec4::<f32>::new(0.0, 0.0, 1.0, 0.0)],
        }
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.extent.width as f32 / self.extent.height as f32
    }

    pub fn update_placement(&mut self, placement: &Placement) {
        self.view_matrix = fps_view(&placement);
        self.camera_model_matrix = camera_model_matrix(&placement);
        self.recompute_frustum_planes();
    }

    pub fn fovy(&self) -> Angle<f32> {
        self.fovx / self.aspect_ratio()
    }

    pub fn adjust_fovx(&mut self, angle: Angle<f32>) {
        self.fovx = self.fovx + angle;
        if self.fovx.as_radians() > MAX_FOV {
            self.fovx = Angle::from_radians(MAX_FOV);
        }
        if self.fovx.as_radians() < MIN_FOV {
            self.fovx = Angle::from_radians(MIN_FOV);
        }
        self.recompute_frustum_planes();
    }

    pub fn recompute_frustum_planes(&mut self) {
        let (sin_hfovx, cos_hfovx) = (self.fovx  / 2.0).as_radians().sin_cos();
        let (sin_hfovy, cos_hfovy) = (self.fovy()/ 2.0).as_radians().sin_cos();

        // Normals point inwards.
        // These are in 'camera model space', and we need to transform
        //   into world space
        let near = Vec4::new(0.0, 0.0, 1.0, -NEAR_PLANE);
        let far = Vec4::new(0.0, 0.0, -1.0, FAR_PLANE);
        let right = Vec4::new(-cos_hfovx, 0.0, sin_hfovx, 0.0);
        let left = Vec4::new(cos_hfovx, 0.0, sin_hfovx, 0.0);
        let top = Vec4::new(0.0, cos_hfovy, sin_hfovy, 0.0);
        let bottom = Vec4::new(0.0, -cos_hfovy, sin_hfovy, 0.0);

        let transform = |plane: &Vec4<f32>, m: &Mat4<f32>| {
            // FIXME: this could probably be optimized.
            let normal = plane.truncate_w().to_vec4(0.0);
            let point = (plane.truncate_w() * -plane.w).to_vec4(1.0);
            let ws_normal = m * &normal;
            let ws_point = m * &point;
            let d = ws_normal.truncate_w().dot(ws_point.truncate_w());
            Vec4::<f32>::new(ws_normal.x, ws_normal.y, ws_normal.z, -d)
        };

        self.frustum_planes[0] = transform(&near, &self.camera_model_matrix);
        self.frustum_planes[1] = transform(&far, &self.camera_model_matrix);
        self.frustum_planes[2] = transform(&right, &self.camera_model_matrix);
        self.frustum_planes[3] = transform(&left, &self.camera_model_matrix);
        self.frustum_planes[4] = transform(&top, &self.camera_model_matrix);
        self.frustum_planes[5] = transform(&bottom, &self.camera_model_matrix);
    }

    /// This does "View Frustum Culling".  It operates in world space.
    pub fn might_be_visible(&self, point: &Point3<f32>, radius: f32) -> bool {
        for plane in 0..6 {
           // Distance from plane. Negative values are outside the frustum.
            let distance =
                self.frustum_planes[plane].truncate_w().dot(point.0)
                + self.frustum_planes[plane][3];
            // If far enough negative, no part of the object is visible:
            if distance < -radius {
                return false;
            }
        }
        true
    }
}

fn fps_view(placement: &Placement) -> Mat4<f32>
{
    // This is like transforming the camera into its world space position,
    // but doing everything backwards (opposite order of operations, each operation
    // of the minus variety).

    // FIXME: this operator could be optimized
    let operator = {
        // minus yaw (clockwise)
        let q_yaw = NQuat::<f32>::from_axis_angle(
            &(-Y_AXIS_F32),
            &Angle::from_radians(-placement.yaw)
        );
        // minus pitch (counter-clockwise, typically +y)
        let q_pitch = NQuat::<f32>::from_axis_angle(
            &X_AXIS_F32,
            &Angle::from_radians(-placement.pitch)
        );
        q_pitch * q_yaw
    };

    let rot_m3 = Mat3::<f32>::from(operator);
    let rot_m4: Mat4<f32> = rot_m3.as_mat4();

    let mut tr_m4: Mat4<f32> = Mat4::identity();
    tr_m4.set_translation(-placement.position);

    // translate then rotate
    &rot_m4 * &tr_m4
}

// This converts points from Camera-model-space into world-space
fn camera_model_matrix(placement: &Placement) -> Mat4<f32>
{
    use siege_math::{Angle, Y_AXIS_F32, X_AXIS_F32};

    let operator = {
        // pitch (clockwise, typically -y)
        let q_pitch = NQuat::<f32>::from_axis_angle(
            &X_AXIS_F32,
            &Angle::from_radians(placement.pitch)
        );
        // yaw (counterwise)
        let q_yaw = NQuat::<f32>::from_axis_angle(
            &(-Y_AXIS_F32),
            &Angle::from_radians(placement.yaw)
        );
        q_yaw * q_pitch
    };
    let rot_m3 = Mat3::<f32>::from(operator);
    let rot_m4: Mat4<f32> = rot_m3.as_mat4();

    let mut m: Mat4<f32> = &rot_m4 * &Mat4::identity();
    m.set_translation(placement.position);
    m
}
