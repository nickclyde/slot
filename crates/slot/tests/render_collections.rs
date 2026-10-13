#![cfg(target_os = "macos")]

mod common;

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use common::{clocked, tmp_root_with_carts};
use slot::frontend::Frontend;
use slot_gfx::{Compositor, HeadlessSurface, OUT_W};
use slot_input::{Btn, InputSource, Millis, RawEvent};
use slot_power::SimPlatform;
use slot_ui::{edge, recess, Keyboard};

struct Script(VecDeque<Vec<RawEvent>>);

impl InputSource for Script {
    fn poll(&mut self, _now: Millis) -> Vec<RawEvent> {
        self.0.pop_front().unwrap_or_default()
    }
}

fn tap(f: &mut Frontend, input: &mut Script, btn: Btn) {
    input.0.push_back(vec![RawEvent::Down(btn)]);
    f.advance(input);
    input.0.push_back(vec![RawEvent::Up(btn)]);
    f.advance(input);
}

fn at(px: &[u8], x: f32, y: f32) -> [u8; 3] {
    let o = (y as usize * OUT_W as usize + x as usize) * 4;
    [px[o], px[o + 1], px[o + 2]]
}

fn near(c: [u8; 3], want: [f32; 4]) -> bool {
    (0..3).all(|i| (c[i] as f32 - want[i] * 255.0).abs() <= 3.0)
}

fn dump(px: &[u8], name: &str) {
    let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") else {
        return;
    };
    let path = format!("{dir}/collections-{name}.png");
    let file = std::fs::File::create(&path).expect("create png");
    let mut e = png::Encoder::new(std::io::BufWriter::new(file), OUT_W, slot_gfx::OUT_H);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header()
        .expect("png header")
        .write_image_data(px)
        .expect("png data");
}

fn ink(px: &[u8], (x, y, w, h): (f32, f32, f32, f32)) -> usize {
    let mut n = 0;
    for yy in y as usize..(y + h) as usize {
        for xx in x as usize..(x + w) as usize {
            if at(px, xx as f32, yy as f32).iter().all(|c| *c > 0xc0) {
                n += 1;
            }
        }
    }
    n
}

fn faint(px: &[u8], (x, y, w, h): (f32, f32, f32, f32)) -> usize {
    let mut n = 0;
    for yy in y as usize..(y + h) as usize {
        for xx in x as usize..(x + w) as usize {
            if at(px, xx as f32, yy as f32)[0] > 0x40 {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn the_search_keyboard_draws_its_keys_with_the_one_under_the_cursor_lit() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    clocked(d.path());
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(&mut c);
    let mut input = Script(VecDeque::new());
    f.advance(&mut input);
    tap(&mut f, &mut input, Btn::X);
    let start = Instant::now();
    let mut px = Vec::new();
    while start.elapsed() < Duration::from_secs(10) {
        f.compose(&mut c);
        px = c.read_frame();
        if ink(&px, Keyboard::key_rect(1, 0)) > 10 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let (lx, ly, ..) = Keyboard::key_rect(1, 0);
    assert!(near(at(&px, lx + 2.0, ly + 2.0), edge()), "Q is not lit");
    let (ux, uy, ..) = Keyboard::key_rect(1, 1);
    assert!(near(at(&px, ux + 2.0, uy + 2.0), recess()), "W is lit too");
    assert!(
        ink(&px, Keyboard::key_rect(1, 0)) > 10,
        "Q has no letter on it"
    );
    assert!(
        ink(&px, Keyboard::key_rect(3, 7)) > 10,
        "Space has no word on it"
    );

    tap(&mut f, &mut input, Btn::A);
    f.compose(&mut c);
    let typed = c.read_frame();
    let field = (80.0, 26.0, 560.0, 48.0);
    assert!(
        ink(&typed, field) > ink(&px, field),
        "typing Q put nothing in the field"
    );

    for btn in [Btn::B, Btn::Right, Btn::Right, Btn::A] {
        tap(&mut f, &mut input, btn);
    }
    f.compose(&mut c);
    let found = c.read_frame();
    dump(&found, "keyboard");
    assert!(
        ink(&found, (180.0, 112.0, 360.0, 24.0)) > 20,
        "Emerald was not listed under E"
    );
}

#[test]
fn a_recently_played_shelf_draws_gba_and_game_boy_carts_side_by_side_and_names_itself() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    std::fs::write(d.path().join("Games/GB/Tetris.gb"), vec![0u8; 0x8000]).unwrap();
    std::fs::write(
        d.path().join("Config/recent.txt"),
        "GB/Tetris\nGBA/Emerald\nGBA/Fusion\n",
    )
    .unwrap();
    clocked(d.path());
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(&mut c);
    let mut input = Script(VecDeque::new());
    f.advance(&mut input);
    f.compose(&mut c);
    f.advance(&mut input);
    std::thread::sleep(Duration::from_millis(300));
    f.advance(&mut input);
    f.compose(&mut c);
    let px = c.read_frame();
    dump(&px, "recent");
    let band = (
        OUT_W as f32 / 2.0 - 70.0,
        slot_gfx::OUT_H as f32 - 49.0,
        140.0,
        27.0,
    );
    assert!(
        faint(&px, band) > 20,
        "the shelf did not say it was Recently Played"
    );

    let named = faint(&px, band);
    tap(&mut f, &mut input, Btn::Y);
    f.compose(&mut c);
    f.advance(&mut input);
    std::thread::sleep(Duration::from_millis(300));
    f.advance(&mut input);
    f.compose(&mut c);
    let noted = c.read_frame();
    dump(&noted, "favorited");
    let favourited = faint(&noted, band);
    assert!(favourited > 20, "favouriting said nothing in the slot");
    assert_ne!(favourited, named, "the slot still says Recently Played");
}
