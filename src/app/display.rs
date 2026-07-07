use eframe::egui;

use super::App;
use crate::dump::{DiscProfile, SectorStatus};
use crate::theme::ColorTheme;

struct BounceDisc {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub angle: f32,
    pub spin: f32,
    pub radius: f32,
}

#[derive(Default)]
pub(crate) struct DiscState {
    pub(super) velocity: f64,
    pub(super) angle: f64,
    pub(super) visual_angle: f32,
    pub(super) dragging: bool,
    drag_offset: f64,
    prev_drag_angle: f64,
    drag_velocity_samples: Vec<f64>,
    shatter_time: Option<(f64, f64)>,
    hold_start: Option<f64>,
    flip_state: Option<(f64, bool)>,
    pub(super) profile_texture: Option<(DiscProfile, egui::TextureHandle)>,
    prev_error_count: usize,
    click_times: Vec<f64>,
    coin_flips: String,
    pub(super) total_rotations: u128,
    pub(super) rotation_accumulator: f64,
    pub(super) prev_angle: f64,
    pub(super) radius_scale: f32,
    right_drag_start_dist: f32,
    right_drag_start_scale: f32,
    pub(super) win_triggered: bool,
    win_animation: Option<(f64, Vec<BounceDisc>)>,
}

impl App {
    // Rotation speed: segments per unit time, irrational to prevent aliasing
    const DISC_SPEED: f64 = 50.0 * 1.618034;
    pub(super) const DISC_ROTATION_SCALE: f64 = (std::f64::consts::TAU / 100.0) * Self::DISC_SPEED;
    pub(super) const ANGLE_PER_ROTATION: f64 = 100.0 / Self::DISC_SPEED;
    const MAX_VELOCITY_SAMPLES: usize = 3; // Affects dragging flick sensitivity

    // Disc geometry as fractions of outer radius
    const DISC_INNER_RADIUS: f32 = 0.35;
    const DISC_METAL_RING_THICKNESS: f32 = 0.07;
    const DISC_SPINDLE_RADIUS: f32 = 0.15;

    // Load the disc profile image
    fn load_profile_texture(ctx: &egui::Context, profile: DiscProfile) -> Option<egui::TextureHandle> {
        let (name, data) = match profile {
            DiscProfile::CD => ("CD", include_bytes!("../../assets/profile/CD.png").as_slice()),
            DiscProfile::DVD => ("DVD", include_bytes!("../../assets/profile/DVD.png").as_slice()),
            DiscProfile::BD => ("BD", include_bytes!("../../assets/profile/BD.png").as_slice()),
            DiscProfile::HDDVD => ("HDDVD", include_bytes!("../../assets/profile/HDDVD.png").as_slice()),
            DiscProfile::XBOX => ("XBOX", include_bytes!("../../assets/profile/XBOX.png").as_slice()),
            DiscProfile::XBOX360 => ("XBOX360", include_bytes!("../../assets/profile/XBOX360.png").as_slice()),
            DiscProfile::GC => ("GC", include_bytes!("../../assets/profile/GC.png").as_slice()),
            DiscProfile::WII => ("WII", include_bytes!("../../assets/profile/WII.png").as_slice()),
        };
        let img = image::load_from_memory(data).ok()?.to_rgba8();
        let (w, h) = (img.width() as usize, img.height() as usize);
        let pixels: Vec<egui::Color32> =
            img.chunks_exact(4).map(|c| egui::Color32::from_rgba_unmultiplied(255, 255, 255, c[3])).collect();
        Some(ctx.load_texture(name, egui::ColorImage::new([w, h], pixels), egui::TextureOptions::LINEAR))
    }

    // Paint the disc profile image
    fn paint_profile_logo(&mut self, ctx: &egui::Context, painter: &egui::Painter, area: egui::Rect, t: &ColorTheme) {
        let profile = match *self.dump.disc_profile.lock().unwrap() {
            Some(p) => p,
            None => return,
        };
        if self.disc.profile_texture.as_ref().map(|(p, _)| *p) != Some(profile) {
            self.disc.profile_texture = Self::load_profile_texture(ctx, profile).map(|t| (profile, t));
        }
        let tex = match &self.disc.profile_texture {
            Some((_, t)) => t,
            None => return,
        };
        let logo_h = 24.0 as f32;
        let logo_w = logo_h * tex.size()[0] as f32 / tex.size()[1] as f32;
        let rect = egui::Rect::from_min_size(
            egui::pos2(area.right() - logo_w - 8.0, area.top() + 8.0),
            egui::vec2(logo_w, logo_h),
        );
        let tint = t.accent;
        painter.image(tex.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), tint);
    }

    // Update box_colors from current sector states
    fn update_box_colors(
        box_colors: &mut Vec<egui::Color32>,
        sectors: &[SectorStatus],
        sectors_per_box: usize,
        num_boxes: usize,
        hash_pct: Option<u8>,
        t: &ColorTheme,
    ) -> usize {
        if box_colors.len() != num_boxes {
            box_colors.clear();
            box_colors.resize(num_boxes, t.bg_dark);
        }

        let mut error_count = 0usize;
        for i in 0..num_boxes {
            let range_start = i * sectors_per_box;
            let range_end = ((i + 1) * sectors_per_box).min(sectors.len());
            let chunk = &sectors[range_start..range_end];

            let mut has_error = false;
            let mut has_unread = false;
            for s in chunk {
                match s {
                    SectorStatus::Error => {
                        has_error = true;
                        break;
                    }
                    SectorStatus::Unread => {
                        has_unread = true;
                    }
                    SectorStatus::Ok => {}
                }
            }

            if has_error {
                error_count += 1;
            }
            let color = if has_error {
                t.accent_stop
            } else if has_unread {
                t.bg_dark
            } else {
                t.accent_action
            };
            box_colors[i] = color;
        }

        if let Some(pct) = hash_pct {
            let hash_box = (pct as usize * num_boxes) / 100;
            let count = (num_boxes / 100).max(1).min(num_boxes - hash_box);
            for i in hash_box..hash_box + count {
                box_colors[i] = t.accent;
            }
        }

        error_count
    }

    fn paint_corner_ticks(
        painter: &egui::Painter,
        rect: egui::Rect,
        color: egui::Color32,
        len: f32,
        stroke_width: f32,
    ) {
        let s = egui::Stroke::new(stroke_width, color);
        painter.line_segment([rect.left_top(), egui::pos2(rect.left() + len, rect.top())], s);
        painter.line_segment([rect.left_top(), egui::pos2(rect.left(), rect.top() + len)], s);
        painter.line_segment([rect.right_top(), egui::pos2(rect.right() - len, rect.top())], s);
        painter.line_segment([rect.right_top(), egui::pos2(rect.right(), rect.top() + len)], s);
        painter.line_segment([rect.left_bottom(), egui::pos2(rect.left() + len, rect.bottom())], s);
        painter.line_segment([rect.left_bottom(), egui::pos2(rect.left(), rect.bottom() - len)], s);
        painter.line_segment([rect.right_bottom(), egui::pos2(rect.right() - len, rect.bottom())], s);
        painter.line_segment([rect.right_bottom(), egui::pos2(rect.right(), rect.bottom() - len)], s);
    }

    fn compute_sector_layout(rect: egui::Rect, num_sectors: usize) -> Option<(usize, usize, f32, usize)> {
        const MIN_BOX_SIZE: f32 = 8.0;
        const MIN_GAP: f32 = 1.0;
        let rect = rect.shrink(2.0);

        if rect.width() < MIN_BOX_SIZE || rect.height() < MIN_BOX_SIZE || num_sectors == 0 {
            return None;
        }

        let min_step = MIN_BOX_SIZE + MIN_GAP;
        let max_cols = (rect.width() / min_step).floor() as usize;
        let max_rows = (rect.height() / min_step).floor() as usize;
        let max_boxes = max_cols.max(1) * max_rows.max(1);

        let sectors_per_box = if num_sectors <= max_boxes {
            1
        } else {
            1usize << ((num_sectors as f64 / max_boxes as f64).log2().ceil() as u32)
        };
        let num_boxes = num_sectors.div_ceil(sectors_per_box);

        let box_size = {
            let mut lo = MIN_BOX_SIZE;
            let mut hi = rect.width().min(rect.height());
            for _ in 0..32 {
                let mid = (lo + hi) * 0.5;
                let step = mid + MIN_GAP;
                let c = (rect.width() / step).floor().max(1.0) as usize;
                let r = (rect.height() / step).floor().max(1.0) as usize;
                if c * r >= num_boxes {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            lo.floor().max(MIN_BOX_SIZE)
        };

        Some((sectors_per_box, num_boxes, box_size, num_sectors))
    }

    fn paint_sector_map(
        painter: &egui::Painter,
        rect: egui::Rect,
        box_colors: &[egui::Color32],
        sectors_per_box: usize,
        num_boxes: usize,
        box_size: f32,
        t: &ColorTheme,
    ) -> (usize, f32) {
        const MIN_BOX_SIZE: f32 = 8.0;
        const MIN_GAP: f32 = 1.0;
        let rect = rect.shrink(2.0);

        if num_boxes == 0 || box_size < MIN_BOX_SIZE {
            return (0, 0.0);
        }

        let step = box_size + MIN_GAP;
        let cols = (rect.width() / step).floor().max(1.0) as usize;
        let rows = (rect.height() / step).floor().max(1.0) as usize;
        let visible_boxes = cols * rows;
        let count = visible_boxes.min(num_boxes);

        // Paint background rects
        let used_cols = cols.min(count);
        let used_width = used_cols as f32 * step - MIN_GAP;
        let border = 1.0 as f32;
        let full_rows_count = count / cols;
        if full_rows_count > 0 {
            let full_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left() - border, rect.top() - border),
                egui::vec2(used_width + border * 2.0, full_rows_count as f32 * step - MIN_GAP + border * 2.0),
            );
            painter.rect_filled(full_rect, 0.0, t.accent);
        }
        let last_row_count = count % cols;
        if last_row_count > 0 {
            let last_row_width = last_row_count as f32 * step - MIN_GAP;
            let last_row_y = rect.top() + full_rows_count as f32 * step;
            let last_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left() - border, last_row_y - border),
                egui::vec2(last_row_width + border * 2.0, box_size + border * 2.0),
            );
            painter.rect_filled(last_rect, 0.0, t.accent);
        }

        // Batch all boxes into a single mesh using cached colors
        let uv = egui::epaint::WHITE_UV;
        let mut mesh = egui::Mesh::default();
        mesh.vertices.reserve(count * 4);
        mesh.indices.reserve(count * 6);

        for (i, &color) in box_colors[..count].iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = rect.left() + col as f32 * step;
            let y = rect.top() + row as f32 * step;

            let base = mesh.vertices.len() as u32;
            let lt = egui::pos2(x, y);
            let rb = egui::pos2(x + box_size, y + box_size);
            mesh.vertices.push(egui::epaint::Vertex { pos: lt, uv, color });
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(rb.x, lt.y), uv, color });
            mesh.vertices.push(egui::epaint::Vertex { pos: rb, uv, color });
            mesh.vertices.push(egui::epaint::Vertex { pos: egui::pos2(lt.x, rb.y), uv, color });
            mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        painter.add(egui::Shape::mesh(mesh));

        (sectors_per_box, box_size)
    }

    // Build ringcode mesh
    fn append_char_to_mesh(
        mesh: &mut egui::Mesh,
        ch: char,
        center: egui::Pos2,
        height: f32,
        rot: f32,
        color: egui::Color32,
    ) {
        let h = height * 0.5;
        let w = height * 0.3;
        let u = w * 0.3;
        let (cr, sr) = (rot.cos(), rot.sin());

        let lines: &[(f32, f32, f32, f32)] = match ch {
            'R' => &[(-w, -h, -w, h), (-w, h, u, h), (-w, 0.0, u, 0.0), (u, h, u, 0.0), (-u, 0.0, u, -h)],
            'E' => &[(-w, -h, -w, h), (-w, h, w, h), (-w, 0.0, w * 0.7, 0.0), (-w, -h, w, -h)],
            'D' => &[(-w, -h, -w, h), (-w, h, u, h), (u, h, u, -h), (-w, -h, u, -h)],
            'U' => &[(-w, -h, -w, h), (w, -h, w, h), (-w, -h, w, -h)],
            'M' => &[(-w, -h, -w, h), (w, -h, w, h), (-w, h, 0.0, 0.0), (0.0, 0.0, w, h)],
            'P' => &[(-w, -h, -w, h), (-w, h, u, h), (-w, 0.0, u, 0.0), (u, h, u, 0.0)],
            _ => &[(-w, -h, w, -h), (w, -h, w, h), (w, h, -w, h), (-w, h, -w, -h)],
        };

        let w = height * 0.2 * 0.5;
        for &(x1, y1, x2, y2) in lines {
            let (dx, dy) = (x2 - x1, y2 - y1);
            let l = (dx * dx + dy * dy).sqrt();
            if l < 0.001 {
                continue;
            }
            let (ux, uy) = (-dy / l * w, dx / l * w);
            let base = mesh.vertices.len() as u32;
            let uv = egui::epaint::WHITE_UV;
            for &(px, py) in &[(x1 + ux, y1 + uy), (x1 - ux, y1 - uy), (x2 - ux, y2 - uy), (x2 + ux, y2 + uy)] {
                let pos = egui::pos2(center.x + px * cr - py * sr, center.y + px * sr + py * cr);
                mesh.vertices.push(egui::epaint::Vertex { pos, uv, color });
            }
            mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    // Handle mouse events on the disc
    fn handle_disc_interaction(
        &mut self,
        ui: &egui::Ui,
        ctx: &egui::Context,
        disc_center: egui::Pos2,
        disc_radius: f32,
        id_salt: &str,
        t: &ColorTheme,
    ) {
        let disc_rect_area =
            egui::Rect::from_center_size(disc_center, egui::vec2(disc_radius * 2.0, disc_radius * 2.0));
        let resp = ui.interact(disc_rect_area, ui.id().with(id_salt), egui::Sense::click_and_drag());
        let spindle_radius = disc_radius * Self::DISC_SPINDLE_RADIUS;
        let in_circle = resp.interact_pointer_pos().map_or(false, |pos| {
            let dist = (pos - disc_center).length();
            dist <= disc_radius && dist >= spindle_radius
        });

        // Check for shatter animation (blocks all interaction)
        if let Some((start, frozen_angle)) = self.disc.shatter_time {
            let elapsed = (ctx.time() - start) as f32;
            if elapsed > 3.0 {
                self.disc.shatter_time = None;
            } else if elapsed < 0.4 {
                Self::paint_shatter(ui.painter(), disc_center, disc_radius, elapsed, frozen_angle as f32, &t);
                ctx.request_repaint();
                return;
            } else {
                ctx.request_repaint();
                return;
            }
        }

        // Check for flip animation (blocks all interaction)
        if let Some((start, heads)) = self.disc.flip_state {
            let elapsed = (ctx.time() - start) as f32;
            if elapsed > 4.0 {
                self.disc.flip_state = None;
                self.disc.coin_flips.push(if heads { 'H' } else { 'T' });
            } else {
                Self::paint_disc_flip(ui.painter(), disc_center, disc_radius, elapsed, heads, t);
                ctx.request_repaint();
                return;
            }
        }

        if resp.drag_started_by(egui::PointerButton::Primary) && in_circle {
            self.disc.dragging = true;
            self.disc.drag_velocity_samples.clear();
            self.disc.drag_velocity_samples.resize(Self::MAX_VELOCITY_SAMPLES, self.disc.velocity);
            if let Some(pos) = resp.interact_pointer_pos() {
                let mouse_angle = (pos.y - disc_center.y).atan2(pos.x - disc_center.x) as f64;
                self.disc.drag_offset = self.disc.angle - mouse_angle / Self::DISC_ROTATION_SCALE;
                self.disc.prev_drag_angle = self.disc.angle;
            }
        }
        if resp.drag_started_by(egui::PointerButton::Secondary) && in_circle {
            if let Some(pos) = resp.interact_pointer_pos() {
                self.disc.right_drag_start_dist = (pos - disc_center).length();
                self.disc.right_drag_start_scale = self.disc.radius_scale;
            }
        }
        // Track hold-without-move for flip (both primary and secondary)
        if (ctx.input(|i| i.pointer.primary_pressed()) || ctx.input(|i| i.pointer.secondary_pressed())) && in_circle {
            self.disc.hold_start = Some(ctx.time());
        }
        if !ctx.input(|i| i.pointer.primary_down()) && !ctx.input(|i| i.pointer.secondary_down()) {
            self.disc.hold_start = None;
        }
        if resp.dragged() && resp.drag_delta().length() > 0.5 {
            self.disc.hold_start = None;
        }
        if let Some(start) = self.disc.hold_start {
            if ctx.time() - start >= 1.0 {
                self.disc.hold_start = None;
                self.disc.dragging = false;
                let heads = ctx.time().to_bits() & 1 == 0;
                self.disc.flip_state = Some((ctx.time(), heads));
            } else {
                ctx.request_repaint();
            }
        }
        if self.disc.dragging && resp.dragged_by(egui::PointerButton::Primary) {
            if resp.drag_delta().length() > 0.5 {
                self.disc.hold_start = None;
            }
            if let Some(pos) = resp.interact_pointer_pos() {
                let dt = ctx.input(|i| i.stable_dt) as f64;
                let mouse_angle = (pos.y - disc_center.y).atan2(pos.x - disc_center.x) as f64;
                if dt > 0.0 {
                    let dx = resp.drag_delta().x as f64;
                    let dy = resp.drag_delta().y as f64;
                    let tx = -(mouse_angle as f64).sin();
                    let ty = (mouse_angle as f64).cos();
                    let tangential_px = dx * tx + dy * ty;
                    let sample = tangential_px / (disc_radius as f64 * Self::DISC_ROTATION_SCALE * dt);
                    if self.disc.drag_velocity_samples.len() >= Self::MAX_VELOCITY_SAMPLES {
                        self.disc.drag_velocity_samples.remove(0);
                    }
                    self.disc.drag_velocity_samples.push(sample);
                    // Additive: blend drag input into current velocity
                    self.disc.velocity += sample - self.disc.velocity;
                }
                self.disc.angle += self.disc.velocity * dt;
            }
            ctx.request_repaint();
        }
        // Right-click drag: resize disc based on radial movement from drag start
        if resp.dragged_by(egui::PointerButton::Secondary) {
            if let Some(pos) = resp.interact_pointer_pos() {
                let dist = (pos - disc_center).length();
                let delta = (dist - self.disc.right_drag_start_dist) / (disc_radius * 3.0);
                self.disc.radius_scale = (self.disc.right_drag_start_scale + delta).clamp(0.15, 3.0);
                ctx.request_repaint();
            }
        }
        if resp.drag_stopped() {
            self.disc.dragging = false;
            self.disc.hold_start = None;
            if !self.disc.drag_velocity_samples.is_empty() {
                let avg =
                    self.disc.drag_velocity_samples.iter().sum::<f64>() / self.disc.drag_velocity_samples.len() as f64;
                self.disc.velocity += avg;
            }
        } else if (resp.clicked() || resp.secondary_clicked()) && in_circle {
            let sign = if self.disc.velocity >= 0.0 { 1.0 } else { -1.0 };
            let bump = 1.0 + self.disc.velocity.abs() * 0.25;
            if resp.secondary_clicked() {
                self.disc.velocity -= bump * sign;
            } else {
                self.disc.velocity += 2.0 * bump * sign;
            }

            // Track rapid left clicks for shatter (right clicks subtract)
            let now = ctx.time();
            self.disc.click_times.retain(|&t| now - t < 2.0);
            if resp.secondary_clicked() {
                self.disc.click_times.pop();
            } else {
                self.disc.click_times.push(now);
            }
        }

        // Scroll wheel rotates the disc
        let hover_in_circle = ctx.pointer_latest_pos().map_or(false, |pos| {
            let dist = (pos - disc_center).length();
            dist <= disc_radius && dist >= spindle_radius
        });
        if hover_in_circle {
            let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.0 {
                self.disc.velocity += scroll as f64 * 0.01;
            }
        }

        // Check for shatter trigger
        if self.disc.shatter_time.is_none() && !self.disc.click_times.is_empty() {
            let len = self.disc.click_times.len();
            let threshold = 11
                + (self.disc.velocity > 1000.0) as usize
                + (self.disc.velocity > 10000.0) as usize
                + (self.disc.click_times[0].to_bits() & 3) as usize;
            let shatter = len >= threshold;
            if shatter {
                self.disc.shatter_time = Some((ctx.time(), self.disc.angle));
                self.disc.velocity = 0.0;
                self.disc.click_times.clear();
                self.disc.coin_flips.clear();
                self.disc.total_rotations = 0;
                self.disc.rotation_accumulator = 0.0;
                self.disc.win_triggered = false;
                self.disc.win_animation = None;
            }
        }

        // Treat 0.0 or negative as 1.0 (uninitialized default)
        if self.disc.radius_scale <= 0.0 {
            self.disc.radius_scale = 1.0;
        }

        // Decay radius_scale back to 1.0 when not right-dragging (exponential decay)
        if !resp.dragged_by(egui::PointerButton::Secondary) && self.disc.radius_scale != 1.0 {
            if (self.disc.radius_scale - 1.0).abs() < 0.001 {
                self.disc.radius_scale = 1.0;
            } else {
                let dt = ctx.input(|i| i.stable_dt);
                let decay = 0.95 as f32;
                self.disc.radius_scale = 1.0 + (self.disc.radius_scale - 1.0) * decay.powf(dt * 60.0);
                ctx.request_repaint();
            }
        }

        let effective_radius = disc_radius * self.disc.radius_scale;
        Self::paint_disc(ui.painter(), disc_center, effective_radius, self.disc.visual_angle, t);
    }

    // Flipping animation
    fn paint_disc_flip(
        painter: &egui::Painter,
        center: egui::Pos2,
        radius: f32,
        elapsed: f32,
        heads: bool,
        t: &ColorTheme,
    ) {
        let tc = elapsed.min(2.5);
        let spin = (10.0 * tc * (1.0 - tc / 8.0) + if heads { 0.0 } else { std::f32::consts::PI }).cos();
        let ry = (radius * spin.abs()).max(2.0);
        let y = center.y - tc * (2.5 - tc) * 80.0;
        let c = egui::pos2(center.x, y);
        let tails = spin < 0.0;

        // Use same dimensions as paint_disc, but scale down over time
        let scale = 1.0 - (tc / 2.5) * 0.5; // Shrink to 50% by tc=2.5
        let inner_radius = radius * Self::DISC_INNER_RADIUS * scale;
        let metal_ring_outer = inner_radius + radius * Self::DISC_METAL_RING_THICKNESS * scale;
        let spindle_radius = radius * Self::DISC_SPINDLE_RADIUS * scale;
        let disc_radius = radius * scale;
        let squish = ry / radius;

        // Disc colors based on which side is showing
        let (r1, g1, b1) = (t.accent.r() as f32, t.accent.g() as f32, t.accent.b() as f32);
        let (r2, g2, b2) = (t.bg_dark.r() as f32, t.bg_dark.g() as f32, t.bg_dark.b() as f32);
        let mix = if tails { 0.85 } else { 0.15 };
        let disc_color = egui::Color32::from_rgb(
            (r1 * (1.0 - mix) + r2 * mix) as u8,
            (g1 * (1.0 - mix) + g2 * mix) as u8,
            (b1 * (1.0 - mix) + b2 * mix) as u8,
        );

        // Metal ring color
        let metal_mix = if tails { 0.6 } else { 0.4 };
        let metal_color = egui::Color32::from_rgb(
            (r1 * (1.0 - metal_mix) + 140.0 * metal_mix) as u8,
            (g1 * (1.0 - metal_mix) + 140.0 * metal_mix) as u8,
            (b1 * (1.0 - metal_mix) + 140.0 * metal_mix) as u8,
        );

        // Draw outer disc (full radius)
        painter.add(egui::Shape::Ellipse(egui::epaint::EllipseShape::filled(
            c,
            egui::vec2(disc_radius, ry * scale),
            disc_color,
        )));

        // Draw metal ring (inner_radius to metal_ring_outer, squished)
        painter.add(egui::Shape::Ellipse(egui::epaint::EllipseShape::filled(
            c,
            egui::vec2(metal_ring_outer, metal_ring_outer * squish),
            metal_color,
        )));

        // Draw inner area (from spindle to inner_radius) with background color
        painter.add(egui::Shape::Ellipse(egui::epaint::EllipseShape::filled(
            c,
            egui::vec2(inner_radius, inner_radius * squish),
            t.bg_dark,
        )));

        // Punch out the spindle hole
        let spindle_ry = spindle_radius * squish;
        painter.add(egui::Shape::Ellipse(egui::epaint::EllipseShape::filled(
            c,
            egui::vec2(spindle_radius, spindle_ry.max(1.0)),
            t.bg_dark,
        )));

        // Outlines matching paint_disc
        painter.add(egui::Shape::Ellipse(egui::epaint::EllipseShape::stroke(
            c,
            egui::vec2(disc_radius, ry * scale),
            egui::Stroke::new(1.5, t.border),
        )));
        if inner_radius * squish > 3.0 {
            painter.add(egui::Shape::Ellipse(egui::epaint::EllipseShape::stroke(
                c,
                egui::vec2(inner_radius, inner_radius * squish),
                egui::Stroke::new(0.5, t.border),
            )));
        }
        if spindle_ry > 2.0 {
            painter.add(egui::Shape::Ellipse(egui::epaint::EllipseShape::stroke(
                c,
                egui::vec2(spindle_radius, spindle_ry),
                egui::Stroke::new(1.0, t.border),
            )));
        }
    }

    // Shatter animation
    fn paint_shatter(painter: &egui::Painter, center: egui::Pos2, radius: f32, t: f32, angle: f32, theme: &ColorTheme) {
        let n = 9 + ((angle.abs() * 100.0) as usize % 9);
        let angle = angle % std::f32::consts::TAU;
        let alpha = ((1.0 - t * 2.5).max(0.0) * 255.0) as u8;
        let color = egui::Color32::from_rgba_unmultiplied(theme.accent.r(), theme.accent.g(), theme.accent.b(), alpha);
        let uv = egui::epaint::WHITE_UV;
        let mut mesh = egui::Mesh::with_texture(egui::TextureId::default());
        for i in 0..n {
            let hash = ((i as f32 + 1.0) * 2654.435) % 1.0;
            let a = angle + (i as f32 + hash * 0.5) * std::f32::consts::TAU / n as f32;
            let size = (0.2 + hash * 0.8) * radius;
            let spread = t * (200.0 + hash * 1400.0);
            let rot = t * (hash - 0.5) * 5.0 + a;
            let c = center + egui::vec2(a.cos(), a.sin()) * spread;
            let (rc, rs) = (rot.cos(), rot.sin());
            let base = mesh.vertices.len() as u32;
            for &(dx, dy) in &[(0.35, 0.0), (1.0, 0.2), (1.0, -0.2)] {
                let (x, y) = (dx * size, dy * size);
                let pos = c + egui::vec2(x * rc - y * rs, x * rs + y * rc);
                mesh.vertices.push(egui::epaint::Vertex { pos, uv, color });
            }
            mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
        }
        painter.add(egui::Shape::mesh(mesh));
    }

    // Paint disc mesh
    fn paint_disc(painter: &egui::Painter, center: egui::Pos2, outer_radius: f32, angle: f32, t: &ColorTheme) {
        // Number of annular sectors to split the disc into
        const NUM_SEGMENTS: usize = 120;
        let arc = std::f32::consts::TAU / NUM_SEGMENTS as f32;

        let inner_radius = outer_radius * Self::DISC_INNER_RADIUS;
        let metal_ring_thickness = outer_radius * Self::DISC_METAL_RING_THICKNESS;
        let metal_ring_outer = inner_radius + metal_ring_thickness;

        // Use theme's accent color to get color range to cycle between
        let (dark, light) = if t.bg_dark.r() > 128 {
            (t.accent, egui::Color32::from_rgb(170, 170, 170))
        } else {
            (egui::Color32::from_rgb(90, 90, 90), t.accent)
        };
        let dark_rgb = [dark.r() as f32, dark.g() as f32, dark.b() as f32];
        let light_rgb = [light.r() as f32, light.g() as f32, light.b() as f32];

        // Precompute cos/sin for cyclic position of the segments
        let (arc_cos, arc_sin) = (arc.cos(), arc.sin());
        let mut pos_cos = angle.cos();
        let mut pos_sin = angle.sin();

        // Precompute cos/sin for brightness rotation of the segments, 2 lobes at 0.5x speed
        let (shimmer_dc, shimmer_ds) = ((arc * 2.0).cos(), (arc * 2.0).sin());
        let mut shim_cos = (angle * 0.5).cos();
        let mut shim_sin = (angle * 0.5).sin();

        // Precompute cos/sin for color rotation of the segments, 5 lobes at 0.3x speed
        let (hue_dc, hue_ds) = ((arc * 5.0).cos(), (arc * 5.0).sin());
        let mut hue_cos = (angle * 0.3).cos();
        let mut hue_sin = (angle * 0.3).sin();

        // Green channel is 120 deg (2.094 rad) offset from red
        let (g_off_cos, g_off_sin) = ((2.094f32).cos(), (2.094f32).sin());
        // Blue channel is 240 deg (4.189 rad) offset from red
        let (b_off_cos, b_off_sin) = ((4.189f32).cos(), (4.189f32).sin());

        // Create disc as a mesh of trapezoids
        let mut mesh = egui::Mesh::default();
        mesh.vertices.reserve(NUM_SEGMENTS * 8);
        mesh.indices.reserve(NUM_SEGMENTS * 12);
        let uv = egui::epaint::WHITE_UV;

        // Paint each segment
        for i in 0..NUM_SEGMENTS {
            // Step segment rotation
            let (c0, s0) = (pos_cos, pos_sin);
            pos_cos = c0 * arc_cos - s0 * arc_sin;
            pos_sin = s0 * arc_cos + c0 * arc_sin;
            let (c1, s1) = (pos_cos, pos_sin);

            // Brightness from shimmer oscillator [0.3, 1.0]
            let brightness = 0.3 + 0.7 * (shim_sin * 0.5 + 0.5);
            let darkness = 1.0 - brightness;

            // Get color offset
            let hue_shift = if t.monochrome { 0.0 } else { 10.0 };
            let hue_r = hue_sin;
            let hue_g = hue_sin * g_off_cos + hue_cos * g_off_sin;
            let hue_b = hue_sin * b_off_cos + hue_cos * b_off_sin;
            let r = (dark_rgb[0] * darkness + light_rgb[0] * brightness + hue_r * hue_shift).clamp(0.0, 255.0) as u8;
            let g = (dark_rgb[1] * darkness + light_rgb[1] * brightness + hue_g * hue_shift).clamp(0.0, 255.0) as u8;
            let b = (dark_rgb[2] * darkness + light_rgb[2] * brightness + hue_b * hue_shift).clamp(0.0, 255.0) as u8;
            let color = egui::Color32::from_rgb(r, g, b);

            // Metal ring color with sharp specular-like shimmer (50% strength)
            let x = shim_sin * 0.5 + 0.5;
            let metal_brightness = x * x * x * x * 0.5;
            let metal_darkness = 1.0 - metal_brightness;
            let metal_r = (dark_rgb[0] * metal_darkness + light_rgb[0] * metal_brightness + hue_r * hue_shift * 0.5)
                .clamp(0.0, 255.0) as u8;
            let metal_g = (dark_rgb[1] * metal_darkness + light_rgb[1] * metal_brightness + hue_g * hue_shift * 0.5)
                .clamp(0.0, 255.0) as u8;
            let metal_b = (dark_rgb[2] * metal_darkness + light_rgb[2] * metal_brightness + hue_b * hue_shift * 0.5)
                .clamp(0.0, 255.0) as u8;
            let metal_color = egui::Color32::from_rgb(metal_r, metal_g, metal_b);

            // Create the main disc quad (from metal ring outer to outer radius)
            mesh.vertices.push(egui::epaint::Vertex {
                pos: center + egui::vec2(c0 * metal_ring_outer, s0 * metal_ring_outer),
                uv,
                color,
            });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: center + egui::vec2(c0 * outer_radius, s0 * outer_radius),
                uv,
                color,
            });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: center + egui::vec2(c1 * outer_radius, s1 * outer_radius),
                uv,
                color,
            });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: center + egui::vec2(c1 * metal_ring_outer, s1 * metal_ring_outer),
                uv,
                color,
            });
            let base = (i * 8) as u32;
            mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);

            // Create the metal ring quad (from inner radius to metal ring outer)
            mesh.vertices.push(egui::epaint::Vertex {
                pos: center + egui::vec2(c0 * inner_radius, s0 * inner_radius),
                uv,
                color: metal_color,
            });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: center + egui::vec2(c0 * metal_ring_outer, s0 * metal_ring_outer),
                uv,
                color: metal_color,
            });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: center + egui::vec2(c1 * metal_ring_outer, s1 * metal_ring_outer),
                uv,
                color: metal_color,
            });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: center + egui::vec2(c1 * inner_radius, s1 * inner_radius),
                uv,
                color: metal_color,
            });
            mesh.indices.extend_from_slice(&[base + 4, base + 5, base + 6, base + 4, base + 6, base + 7]);

            // Step brightness rotation
            let (sc, ss) = (shim_cos, shim_sin);
            shim_cos = sc * shimmer_dc - ss * shimmer_ds;
            shim_sin = ss * shimmer_dc + sc * shimmer_ds;

            // Step color rotation
            let (hc, hs) = (hue_cos, hue_sin);
            hue_cos = hc * hue_dc - hs * hue_ds;
            hue_sin = hs * hue_dc + hc * hue_ds;
        }

        // Paint the disc
        painter.add(egui::Shape::mesh(mesh));

        // Add REDUMPER ringcode (mirrored)
        let text_radius = (inner_radius + metal_ring_outer) * 0.5;
        let font_height = metal_ring_thickness * 0.5;

        let char_spacing = 0.15 as f32;
        let mut text_mesh = egui::Mesh::default();

        for (i, ch) in "REDUMPER".chars().enumerate() {
            let char_angle = angle + (i as f32 * char_spacing);
            let char_center =
                egui::pos2(center.x + char_angle.cos() * text_radius, center.y + char_angle.sin() * text_radius);
            Self::append_char_to_mesh(
                &mut text_mesh,
                ch,
                char_center,
                font_height,
                char_angle + std::f32::consts::FRAC_PI_2,
                t.bg_dark,
            );
        }
        painter.add(egui::Shape::mesh(text_mesh));

        // Also paint the disc outlines (outer, inner, spindle)
        painter.circle_stroke(center, outer_radius, egui::Stroke::new(1.5, t.border));
        painter.circle_stroke(center, inner_radius, egui::Stroke::new(0.5, t.border));
        painter.circle_stroke(center, outer_radius * Self::DISC_SPINDLE_RADIUS, egui::Stroke::new(1.0, t.border));
    }

    // Paint the central display panel
    pub(super) fn paint_display(&mut self, ui: &mut egui::Ui, t: &ColorTheme, is_running: bool) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(t.bg_dark)
                    .inner_margin(egui::Margin::same(8))
                    .stroke(egui::Stroke::new(1.0, t.border)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("DISPLAY").color(t.text_dim).size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if is_running {
                            ui.label(egui::RichText::new("● ACTIVE").color(t.accent_stop).monospace().size(12.0));
                        } else if self.dump_just_finished {
                            ui.label(egui::RichText::new("● COMPLETE").color(t.accent_action).monospace().size(12.0));
                        } else {
                            ui.label(egui::RichText::new("○ STANDBY").color(t.text_dim).monospace().size(12.0));
                        }
                    });
                });

                let available = ui.available_rect_before_wrap();
                if available.height() > 50.0 {
                    Self::paint_corner_ticks(ui.painter(), available.shrink(6.0), t.border, 12.0, 1.0);
                    let available = available.shrink(5.0);
                    // If height is sufficient, give disc a square region on the right
                    let show_disc = available.height() >= 160.0;
                    let (map_rect, disc_rect) = if show_disc {
                        let disc_side = available.height().min(available.width() * 0.6);
                        let dr = egui::Rect::from_min_size(
                            egui::pos2(available.right() - disc_side, available.top()),
                            egui::vec2(disc_side, available.height()),
                        );
                        let mr = egui::Rect::from_min_max(
                            available.left_top() + egui::vec2(8.0, 8.0),
                            egui::pos2(dr.left() - 8.0, available.bottom() - 8.0),
                        );
                        (mr, dr)
                    } else {
                        let mr = egui::Rect::from_min_max(
                            available.left_top() + egui::vec2(8.0, 8.0),
                            available.right_bottom() - egui::vec2(8.0, 8.0),
                        );
                        (mr, egui::Rect::NOTHING)
                    };

                    // Reserve bottom row for legend
                    let legend_height = 14.0;
                    let legend_rect = egui::Rect::from_min_max(
                        egui::pos2(available.left() + 8.0, available.bottom() - legend_height - 6.0),
                        egui::pos2(available.right() - 8.0, available.bottom() - 6.0),
                    );
                    let map_rect = egui::Rect::from_min_max(
                        map_rect.left_top(),
                        egui::pos2(map_rect.right(), legend_rect.top() - 4.0),
                    );

                    // Paint sector map and legend only if there are sectors
                    let sectors = self.dump.sector_states.lock().unwrap_or_else(|e| e.into_inner());
                    let has_sectors = !sectors.is_empty();
                    if has_sectors {
                        // Compute sector counts for summary (when dump is complete or partial dump exists)
                        let sector_counts = if self.dump_just_finished || self.dump_refinable {
                            let mut good = 0usize;
                            let mut bad = 0usize;
                            let mut unread = 0usize;
                            for s in sectors.iter() {
                                match s {
                                    SectorStatus::Ok => good += 1,
                                    SectorStatus::Error => bad += 1,
                                    SectorStatus::Unread => unread += 1,
                                }
                            }
                            Some((good, bad, unread))
                        } else {
                            None
                        };

                        let (sectors_per_box, box_size) = if let Some((spb, num_boxes, bsz, _)) =
                            Self::compute_sector_layout(map_rect, sectors.len())
                        {
                            let hash_pct = self.dump.hash_progress.lock().ok().and_then(|g| *g);
                            let error_count =
                                Self::update_box_colors(&mut self.box_colors, &sectors, spb, num_boxes, hash_pct, t);
                            drop(sectors);
                            self.disc.prev_error_count = error_count;
                            Self::paint_sector_map(ui.painter(), map_rect, &self.box_colors, spb, num_boxes, bsz, t)
                        } else {
                            drop(sectors);
                            (0, 0.0)
                        };

                        // Paint legend if sector map has been painted
                        if sectors_per_box > 0 && box_size > 0.0 {
                            let painter = ui.painter();
                            let mut x = legend_rect.left();
                            let cy = legend_rect.center().y;
                            let bsz = box_size;
                            let font = egui::FontId::monospace(11.0);

                            let galley = painter.layout_no_wrap(
                                format!("{} SECTORS EACH: ", sectors_per_box),
                                font.clone(),
                                t.text_dim,
                            );
                            painter.galley(egui::pos2(x, cy - galley.size().y * 0.5), galley.clone(), t.text_dim);
                            x += galley.size().x;

                            let br = egui::Rect::from_center_size(egui::pos2(x + bsz * 0.5, cy), egui::vec2(bsz, bsz));
                            painter.rect_stroke(br, 0.0, egui::Stroke::new(0.5, t.accent), egui::StrokeKind::Inside);
                            x += bsz + 3.0;
                            let galley = painter.layout_no_wrap("UNREAD ".to_string(), font.clone(), t.text_dim);
                            painter.galley(egui::pos2(x, cy - galley.size().y * 0.5), galley.clone(), t.text_dim);
                            x += galley.size().x;

                            let br = egui::Rect::from_center_size(egui::pos2(x + bsz * 0.5, cy), egui::vec2(bsz, bsz));
                            painter.rect_filled(br, 0.0, t.accent_action);
                            x += bsz + 3.0;
                            let galley = painter.layout_no_wrap("GOOD ".to_string(), font.clone(), t.text_dim);
                            painter.galley(egui::pos2(x, cy - galley.size().y * 0.5), galley.clone(), t.text_dim);
                            x += galley.size().x;

                            let br = egui::Rect::from_center_size(egui::pos2(x + bsz * 0.5, cy), egui::vec2(bsz, bsz));
                            painter.rect_filled(br, 0.0, t.accent_stop);
                            x += bsz + 3.0;
                            let galley = painter.layout_no_wrap("BAD".to_string(), font.clone(), t.text_dim);
                            painter.galley(egui::pos2(x, cy - galley.size().y * 0.5), galley, t.text_dim);

                            // Right-aligned sector counts when dump is complete
                            if let Some((good, bad, unread)) = sector_counts {
                                let summary = format!("GOOD: {}  BAD: {}  UNREAD: {}", good, bad, unread);
                                let galley = painter.layout_no_wrap(summary, font, t.text_dim);
                                let rx = legend_rect.right() - galley.size().x;
                                painter.galley(egui::pos2(rx, cy - galley.size().y * 0.5), galley, t.text_dim);
                            }
                        }
                        // Paint disc in its designated area
                        if show_disc {
                            let disc_radius = disc_rect.height().min(disc_rect.width()) * 0.4;
                            let disc_center = egui::pos2(disc_rect.center().x, disc_rect.top() + disc_radius + 8.0);
                            if self.disc.velocity.abs() > 0.001 {
                                ctx.request_repaint();
                            }
                            self.handle_disc_interaction(ui, &ctx, disc_center, disc_radius, "disc_spin", t);
                        }
                    } else {
                        drop(sectors);

                        // No sectors: center the disc in the full available area
                        let disc_radius = available.height().min(available.width()) * 0.43;
                        let disc_center = available.center();
                        if self.disc.velocity.abs() > 0.001 {
                            ctx.request_repaint();
                        }
                        self.handle_disc_interaction(ui, &ctx, disc_center, disc_radius, "disc_spin2", t);

                        // Paint coin flip history in bottom-left
                        if !self.disc.coin_flips.is_empty() {
                            let painter = ui.painter();
                            let pos = egui::pos2(available.left() + 12.0, available.bottom() - 6.0);
                            painter.text(
                                pos,
                                egui::Align2::LEFT_BOTTOM,
                                &self.disc.coin_flips,
                                egui::FontId::monospace(13.0),
                                t.text_dim,
                            );
                        }

                        // Paint win animation
                        if let Some((start, ref mut discs)) = self.disc.win_animation {
                            let dt = ctx.input(|i| i.stable_dt);
                            let floor = available.bottom();
                            let painter = ui.painter();
                            for d in discs.iter_mut() {
                                Self::paint_disc(painter, egui::pos2(d.x, d.y), d.radius, d.angle, t);
                                d.x += d.vx * dt;
                                d.vy += 600.0 as f32 * dt;
                                d.y += d.vy * dt;
                                d.angle += d.spin * dt;
                                if d.y + d.radius > floor {
                                    d.y = floor - d.radius;
                                    d.vy = -d.vy * 0.85;
                                }
                            }
                            // Continue the animation until all discs are off-screen (or 12sec total)
                            if ctx.time() - start < 12.0
                                && discs
                                    .iter()
                                    .any(|d| d.x > available.left() - d.radius && d.x < available.right() + d.radius)
                            {
                                ctx.request_repaint();
                            } else {
                                // Stop the animation
                                self.disc.win_animation = None;
                            }
                        }

                        // Trigger win animation once when rotations overflow
                        if self.disc.total_rotations == u128::MAX && !self.disc.win_triggered {
                            // Reset disc state
                            self.disc.velocity = 0.0;
                            self.disc.angle = 0.0;
                            self.disc.total_rotations = 0;
                            self.disc.rotation_accumulator = 0.0;

                            // Setup the animation
                            let discs: Vec<BounceDisc> = (0..12)
                                .map(|i| {
                                    // Every 2nd disc goes left
                                    let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
                                    // Create the child disc
                                    BounceDisc {
                                        x: disc_center.x,
                                        y: disc_center.y,
                                        vx: sign * (80.0 + i as f32 * 30.0),
                                        vy: -(100.0 + i as f32 * 40.0),
                                        angle: i as f32 * 0.9,
                                        spin: sign * (2.0 + i as f32 * 0.5),
                                        radius: disc_radius * 0.4,
                                    }
                                })
                                .collect();

                            // Start the animation timer
                            self.disc.win_animation = Some((ctx.time(), discs));
                            self.disc.win_triggered = true;
                        }

                        // Paint rotation counter or win text in bottom-right
                        if self.disc.win_triggered {
                            let painter = ui.painter();
                            let pos = egui::pos2(available.right() - 12.0, available.bottom() - 6.0);
                            painter.text(
                                pos,
                                egui::Align2::RIGHT_BOTTOM,
                                "YOU WIN",
                                egui::FontId::monospace(13.0),
                                t.text_dim,
                            );
                        } else if self.disc.total_rotations > 0 {
                            let painter = ui.painter();
                            let pos = egui::pos2(available.right() - 12.0, available.bottom() - 6.0);
                            painter.text(
                                pos,
                                egui::Align2::RIGHT_BOTTOM,
                                format!("{}", self.disc.total_rotations),
                                egui::FontId::monospace(13.0),
                                t.text_dim,
                            );

                            // Show CD speed in top-right when spinning fast
                            let rpm = (self.disc.velocity / Self::ANGLE_PER_ROTATION * 60.0).abs();
                            let cd_speed = rpm / 200.0;
                            if cd_speed >= 1.0 {
                                painter.text(
                                    egui::pos2(available.right() - 12.0, available.top() + 6.0),
                                    egui::Align2::RIGHT_TOP,
                                    format!("{:.0}x", cd_speed),
                                    egui::FontId::monospace(12.0),
                                    t.text_dim,
                                );
                            }
                        }
                    }

                    // Profile logo at top-right of disc area
                    if show_disc && available.width() >= 600.0 {
                        self.paint_profile_logo(&ctx, ui.painter(), disc_rect, t);
                    }
                }
            });
    }
}
