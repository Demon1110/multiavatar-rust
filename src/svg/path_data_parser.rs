//! SVG path 数据解析器：把 `d` 属性解析为 tiny-skia 的 [Path]。
//!
//! 支持指令：M/m、L/l、H/h、V/v、C/c、S/s、Q/q、T/t、A/a、Z/z；
//! 支持隐式指令重复（如 `M a b c d` 中后一对按 L 处理）、逗号/空白分隔、
//! 符号与小数点前缀以及科学计数法（如 `4e-5`）。
//! 圆弧（A/a）按 W3C SVG 1.1 附录 F.6 算法转换为三次贝塞尔曲线。
//! 填充规则固定为 Winding（NonZero），与 Java Path2D WIND_NON_ZERO 一致。

use tiny_skia::PathBuilder;

/// 解析 path 数据。
///
/// 返回 `(Path, 是否有有效几何)`；d 为 null/空 返回空 Path。
pub fn parse(d: &str) -> (tiny_skia::Path, bool) {
    let mut pb = PathBuilder::new();
    if d.is_empty() {
        return (empty_path(), false);
    }
    let mut parser = Parser {
        s: d,
        bytes: d.as_bytes(),
        i: 0,
        cx: 0.0,
        cy: 0.0,
        sx: 0.0,
        sy: 0.0,
        prev_cubic_x: 0.0,
        prev_cubic_y: 0.0,
        prev_quad_x: 0.0,
        prev_quad_y: 0.0,
        last_cmd: 0,
        moved: false,
        pb: &mut pb,
    };
    parser.run();
    let moved = parser.moved;
    (pb.finish().unwrap_or_else(empty_path), moved)
}

/// 空路径占位（moved=false 时不会被使用）。
fn empty_path() -> tiny_skia::Path {
    let mut pb = PathBuilder::new();
    pb.move_to(0.0, 0.0);
    pb.finish().expect("empty path 构造失败")
}

/// F.6.5 的 angle 函数（atan2 归一化到 [0, 2π)）。
fn angle(ux: f64, uy: f64, vx: f64, vy: f64) -> f64 {
    let dot = ux * vx + uy * vy;
    let len = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
    let a = (dot / len).clamp(-1.0, 1.0).acos();
    if ux * vy - uy * vx < 0.0 {
        -a
    } else {
        a
    }
}

fn is_letter(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

struct Parser<'a> {
    s: &'a str,
    bytes: &'a [u8],
    i: usize,
    cx: f64,
    cy: f64,
    sx: f64,
    sy: f64,
    prev_cubic_x: f64,
    prev_cubic_y: f64,
    prev_quad_x: f64,
    prev_quad_y: f64,
    last_cmd: u8,
    moved: bool,
    pb: &'a mut PathBuilder,
}

impl Parser<'_> {
    fn run(&mut self) {
        while self.i < self.bytes.len() {
            self.skip_sep();
            if self.i >= self.bytes.len() {
                break;
            }
            let c = self.bytes[self.i];
            let cmd: u8;
            if is_letter(c) {
                cmd = c;
                self.i += 1;
            } else if self.last_cmd == 0 {
                break; // 没有起始指令，忽略
            } else if self.last_cmd == b'M' {
                cmd = b'L';
            } else if self.last_cmd == b'm' {
                cmd = b'l';
            } else {
                cmd = self.last_cmd; // 隐式重复上一指令
            }
            self.exec(cmd);
            self.last_cmd = cmd;
        }
    }

    fn exec(&mut self, cmd: u8) {
        match cmd {
            b'M' => {
                let x = self.read_num();
                let y = self.read_num();
                self.move_to(x, y, false);
            }
            b'm' => {
                let x = self.read_num();
                let y = self.read_num();
                self.move_to(x, y, true);
            }
            b'L' => {
                let x = self.read_num();
                let y = self.read_num();
                self.line_to(x, y, false);
            }
            b'l' => {
                let x = self.read_num();
                let y = self.read_num();
                self.line_to(x, y, true);
            }
            b'H' => {
                let x = self.read_num();
                self.h_line(x, false);
            }
            b'h' => {
                let x = self.read_num();
                self.h_line(x, true);
            }
            b'V' => {
                let y = self.read_num();
                self.v_line(y, false);
            }
            b'v' => {
                let y = self.read_num();
                self.v_line(y, true);
            }
            b'C' => {
                let x1 = self.read_num();
                let y1 = self.read_num();
                let x2 = self.read_num();
                let y2 = self.read_num();
                let x = self.read_num();
                let y = self.read_num();
                self.cubic(x1, y1, x2, y2, x, y, false);
            }
            b'c' => {
                let x1 = self.read_num();
                let y1 = self.read_num();
                let x2 = self.read_num();
                let y2 = self.read_num();
                let x = self.read_num();
                let y = self.read_num();
                self.cubic(x1, y1, x2, y2, x, y, true);
            }
            b'S' => {
                let x2 = self.read_num();
                let y2 = self.read_num();
                let x = self.read_num();
                let y = self.read_num();
                self.smooth_cubic(x2, y2, x, y, false);
            }
            b's' => {
                let x2 = self.read_num();
                let y2 = self.read_num();
                let x = self.read_num();
                let y = self.read_num();
                self.smooth_cubic(x2, y2, x, y, true);
            }
            b'Q' => {
                let x1 = self.read_num();
                let y1 = self.read_num();
                let x = self.read_num();
                let y = self.read_num();
                self.quad(x1, y1, x, y, false);
            }
            b'q' => {
                let x1 = self.read_num();
                let y1 = self.read_num();
                let x = self.read_num();
                let y = self.read_num();
                self.quad(x1, y1, x, y, true);
            }
            b'T' => {
                let x = self.read_num();
                let y = self.read_num();
                self.smooth_quad(x, y, false);
            }
            b't' => {
                let x = self.read_num();
                let y = self.read_num();
                self.smooth_quad(x, y, true);
            }
            b'A' => {
                let rx = self.read_num();
                let ry = self.read_num();
                let rot = self.read_num();
                let large = self.read_flag();
                let sweep = self.read_flag();
                let x = self.read_num();
                let y = self.read_num();
                self.arc(rx, ry, rot, large, sweep, x, y, false);
            }
            b'a' => {
                let rx = self.read_num();
                let ry = self.read_num();
                let rot = self.read_num();
                let large = self.read_flag();
                let sweep = self.read_flag();
                let x = self.read_num();
                let y = self.read_num();
                self.arc(rx, ry, rot, large, sweep, x, y, true);
            }
            b'Z' | b'z' => {
                self.pb.close();
                self.cx = self.sx;
                self.cy = self.sy;
            }
            _ => {}
        }
    }

    fn move_to(&mut self, x: f64, y: f64, rel: bool) {
        let (x, y) = if rel { (x + self.cx, y + self.cy) } else { (x, y) };
        self.pb.move_to(x as f32, y as f32);
        self.cx = x;
        self.cy = y;
        self.sx = x;
        self.sy = y;
        self.moved = true;
    }

    fn line_to(&mut self, x: f64, y: f64, rel: bool) {
        let (x, y) = if rel { (x + self.cx, y + self.cy) } else { (x, y) };
        self.pb.line_to(x as f32, y as f32);
        self.cx = x;
        self.cy = y;
    }

    fn h_line(&mut self, x: f64, rel: bool) {
        self.cx = if rel { self.cx + x } else { x };
        self.pb.line_to(self.cx as f32, self.cy as f32);
    }

    fn v_line(&mut self, y: f64, rel: bool) {
        self.cy = if rel { self.cy + y } else { y };
        self.pb.line_to(self.cx as f32, self.cy as f32);
    }

    fn cubic(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64, rel: bool) {
        let (x1, y1, x2, y2, x, y) = if rel {
            (x1 + self.cx, y1 + self.cy, x2 + self.cx, y2 + self.cy, x + self.cx, y + self.cy)
        } else {
            (x1, y1, x2, y2, x, y)
        };
        self.pb.cubic_to(x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32);
        self.prev_cubic_x = x2;
        self.prev_cubic_y = y2;
        self.cx = x;
        self.cy = y;
    }

    fn smooth_cubic(&mut self, x2: f64, y2: f64, x: f64, y: f64, rel: bool) {
        // 反射控制点恒为绝对坐标（相对上一 C/S 的第二控制点关于当前点镜像）
        let (mut x1, mut y1) = (self.cx, self.cy);
        if matches!(self.last_cmd, b'C' | b'c' | b'S' | b's') {
            x1 = 2.0 * self.cx - self.prev_cubic_x;
            y1 = 2.0 * self.cy - self.prev_cubic_y;
        }
        let ax2 = if rel { self.cx + x2 } else { x2 };
        let ay2 = if rel { self.cy + y2 } else { y2 };
        let ax = if rel { self.cx + x } else { x };
        let ay = if rel { self.cy + y } else { y };
        self.pb.cubic_to(x1 as f32, y1 as f32, ax2 as f32, ay2 as f32, ax as f32, ay as f32);
        self.prev_cubic_x = ax2;
        self.prev_cubic_y = ay2;
        self.cx = ax;
        self.cy = ay;
    }

    fn quad(&mut self, x1: f64, y1: f64, x: f64, y: f64, rel: bool) {
        let (x1, y1, x, y) = if rel {
            (x1 + self.cx, y1 + self.cy, x + self.cx, y + self.cy)
        } else {
            (x1, y1, x, y)
        };
        // 与 Java Path2D 一致的二次→三次贝塞尔等价表示
        let cx1 = self.cx + 2.0 / 3.0 * (x1 - self.cx);
        let cy1 = self.cy + 2.0 / 3.0 * (y1 - self.cy);
        let cx2 = x + 2.0 / 3.0 * (x1 - x);
        let cy2 = y + 2.0 / 3.0 * (y1 - y);
        self.pb.cubic_to(cx1 as f32, cy1 as f32, cx2 as f32, cy2 as f32, x as f32, y as f32);
        self.prev_quad_x = x1;
        self.prev_quad_y = y1;
        self.cx = x;
        self.cy = y;
    }

    fn smooth_quad(&mut self, x: f64, y: f64, rel: bool) {
        // 反射控制点恒为绝对坐标
        let (mut x1, mut y1) = (self.cx, self.cy);
        if matches!(self.last_cmd, b'Q' | b'q' | b'T' | b't') {
            x1 = 2.0 * self.cx - self.prev_quad_x;
            y1 = 2.0 * self.cy - self.prev_quad_y;
        }
        let ax = if rel { self.cx + x } else { x };
        let ay = if rel { self.cy + y } else { y };
        let c1x = self.cx + 2.0 / 3.0 * (x1 - self.cx);
        let c1y = self.cy + 2.0 / 3.0 * (y1 - self.cy);
        let c2x = ax + 2.0 / 3.0 * (x1 - ax);
        let c2y = ay + 2.0 / 3.0 * (y1 - ay);
        self.pb.cubic_to(c1x as f32, c1y as f32, c2x as f32, c2y as f32, ax as f32, ay as f32);
        self.prev_quad_x = x1;
        self.prev_quad_y = y1;
        self.cx = ax;
        self.cy = ay;
    }

    /// SVG 圆弧 → 三次贝塞尔（W3C SVG 1.1 F.6 标准实现）。
    fn arc(
        &mut self,
        rx: f64, ry: f64, phi_deg: f64,
        large_arc: bool, sweep: bool,
        x: f64, y: f64, rel: bool,
    ) {
        let x2 = if rel { self.cx + x } else { x };
        let y2 = if rel { self.cy + y } else { y };
        let (x1, y1) = (self.cx, self.cy);

        if x1 == x2 && y1 == y2 {
            return; // 起点=终点，按规范忽略
        }
        let mut rx = rx.abs();
        let mut ry = ry.abs();
        if rx == 0.0 || ry == 0.0 {
            self.pb.line_to(x2 as f32, y2 as f32); // 半径为零退化为直线
            self.cx = x2;
            self.cy = y2;
            return;
        }

        let phi = phi_deg % 360.0 * std::f64::consts::PI / 180.0;
        let cos_phi = phi.cos();
        let sin_phi = phi.sin();

        // F.6.5 步骤 1：变换到椭圆中心坐标系
        let dx2 = (x1 - x2) / 2.0;
        let dy2 = (y1 - y2) / 2.0;
        let x1p = cos_phi * dx2 + sin_phi * dy2;
        let y1p = -sin_phi * dx2 + cos_phi * dy2;

        // F.6.6 步骤 2：半径校正
        let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
        if lambda > 1.0 {
            let sq = lambda.sqrt();
            rx *= sq;
            ry *= sq;
        }

        // F.6.5 步骤 3：计算中心坐标（单位圆中）
        let rx2 = rx * rx;
        let ry2 = ry * ry;
        let num = rx2 * ry2 - rx2 * y1p * y1p - ry2 * x1p * x1p;
        let den = rx2 * y1p * y1p + ry2 * x1p * x1p;
        let coef = if large_arc != sweep { 1.0 } else { -1.0 }
            * (num / den).max(0.0).sqrt();
        let cxp = coef * (rx * y1p / ry);
        let cyp = coef * (-ry * x1p / rx);

        // F.6.5 步骤 4：变换回原坐标系
        let center_x = cos_phi * cxp - sin_phi * cyp + (x1 + x2) / 2.0;
        let center_y = sin_phi * cxp + cos_phi * cyp + (y1 + y2) / 2.0;

        // F.6.5 步骤 5：起始角与角度增量
        let theta1 = angle(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
        let mut d_theta = angle(
            (x1p - cxp) / rx, (y1p - cyp) / ry,
            (-x1p - cxp) / rx, (-y1p - cyp) / ry,
        );
        if !sweep && d_theta > 0.0 {
            d_theta -= 2.0 * std::f64::consts::PI;
        } else if sweep && d_theta < 0.0 {
            d_theta += 2.0 * std::f64::consts::PI;
        }

        // 分段（每段不超过 90 度）
        let segments = ((d_theta.abs() / (std::f64::consts::PI / 2.0)).ceil() as i64).max(1);
        let seg = d_theta / segments as f64;
        let mut theta = theta1;

        // 弧线从当前点(x1,y1)连续绘制，不新开子路径
        for k in 0..segments {
            let t2 = if k == segments - 1 { theta1 + d_theta } else { theta + seg };
            let (cos_t, sin_t) = (theta.cos(), theta.sin());
            let (cos_t2, sin_t2) = (t2.cos(), t2.sin());

            let alpha = (seg / 2.0).tan();
            let alpha = seg.sin() * ((4.0 + 3.0 * alpha * alpha).sqrt() - 1.0) / 3.0;

            let p1x = center_x + rx * cos_t;
            let p1y = center_y + ry * sin_t;
            let p2x = center_x + rx * cos_t2;
            let p2y = center_y + ry * sin_t2;

            let cp1x = p1x - alpha * rx * sin_t;
            let cp1y = p1y + alpha * ry * cos_t;
            let cp2x = p2x + alpha * rx * sin_t2;
            let cp2y = p2y - alpha * ry * cos_t2;

            self.pb
                .cubic_to(cp1x as f32, cp1y as f32, cp2x as f32, cp2y as f32, p2x as f32, p2y as f32);
            theta = t2;
        }
        self.cx = x2;
        self.cy = y2;
    }

    // ---------- 数字读取 ----------

    fn read_num(&mut self) -> f64 {
        self.skip_sep();
        let start = self.i;
        if self.i < self.bytes.len() && (self.bytes[self.i] == b'-' || self.bytes[self.i] == b'+') {
            self.i += 1;
        }
        let mut dot = false;
        while self.i < self.bytes.len() {
            let c = self.bytes[self.i];
            if c.is_ascii_digit() {
                self.i += 1;
            } else if c == b'.' && !dot {
                dot = true;
                self.i += 1;
            } else if (c == b'e' || c == b'E') && self.i + 1 < self.bytes.len() {
                // 科学计数法
                let n = self.bytes[self.i + 1];
                if n.is_ascii_digit() || n == b'-' || n == b'+' {
                    self.i += 1;
                    if n == b'-' || n == b'+' {
                        self.i += 1;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if self.i == start {
            return 0.0;
        }
        self.s[start..self.i].parse::<f64>().unwrap_or(0.0)
    }

    /// 读取 0/1 标志。
    fn read_flag(&mut self) -> bool {
        self.skip_sep();
        if self.i < self.bytes.len() && (self.bytes[self.i] == b'0' || self.bytes[self.i] == b'1') {
            let f = self.bytes[self.i] == b'1';
            self.i += 1;
            f
        } else {
            false
        }
    }

    fn skip_sep(&mut self) {
        while self.i < self.bytes.len() {
            let c = self.bytes[self.i];
            if c == b' ' || c == b'\t' || c == b'\r' || c == b'\n' || c == b',' {
                self.i += 1;
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_geometry(d: &str) -> bool {
        parse(d).1
    }

    #[test]
    fn parse_moveto_lineto() {
        let (path, moved) = parse("M 10 20 L 30 40");
        assert!(moved);
        let pb = path;
        let _ = pb;
    }

    #[test]
    fn arc_supported() {
        assert!(has_geometry("M 0 0 A 10 10 0 0 1 20 20"));
    }

    #[test]
    fn implicit_repeat() {
        assert!(has_geometry("M 0 0 10 10 20 20"));
    }

    #[test]
    fn relative_commands() {
        assert!(has_geometry("m 0 0 l 10 10 h 5 v -5 c 1 1 2 2 3 3"));
    }

    #[test]
    fn scientific_notation() {
        assert!(has_geometry("M 4e-5 -3e-5 L 1 1"));
    }
}
