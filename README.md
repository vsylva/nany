<h3>&nbsp; <a href="README.md">English</a> | <a href="README.zh-CN.md">中文</a> &nbsp;</h3>

## Overlay framework for **Direct3D 11 / 12 / D3D11on12 / OpenGL / Vulkan**, backend detected automatically per swapchain.

<img src="assets/screenshots/Dying Light.png" width="360" alt="Dying Light"> <img src="assets/screenshots/Resident Evil 4.png" width="360" alt="Resident Evil 4">

<img src="assets/screenshots/Rich Man 11.png" width="360" alt="Rich Man 11"> <img src="assets/screenshots/Rune Factory 3.png" width="360" alt="Rune Factory 3">

<img src="assets/screenshots/Rune Factory 4.png" width="360" alt="Rune Factory 4"> <img src="assets/screenshots/The Evil Within 2.png" width="360" alt="The Evil Within 2">

<img src="assets/screenshots/The Evil Within.png" width="360" alt="The Evil Within"> <img src="assets/screenshots/To the Moon.png" width="360" alt="To the Moon">

## Example

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
nany = { path = "../nany" }
egui = { version = "=0.37.0", default-features = false, features = ["default_fonts"] }
windows = { version = "=0.58.0", default-features = false, features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_WindowsAndMessaging",
] }
```

```rust
use egui;
use nany::overlay::{Frame, Provider};
use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

struct MyMenu {
    hwnd: HWND,
    last: Mutex<[bool; 5]>,
}

impl Provider for MyMenu {
    unsafe fn init(&self, ctx: &egui::Context) {
        nany::ui::install_font(ctx, "C:\\Windows\\Fonts\\msyh.ttc");
        ctx.set_visuals(egui::Visuals::dark());
    }

    unsafe fn ui(&self, f: &Frame<'_>) -> bool {
        // window messages do not work in some games: read the cursor and buttons yourself and feed egui
        let mut p = POINT::default();
        if GetCursorPos(&mut p).is_ok() {
            let mut c = p;
            let _ = ScreenToClient(self.hwnd, &mut c);
            nany::input::on_mouse_move(c.x, c.y);
        }

        let mut last = match self.last.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let now = [
            GetAsyncKeyState(0x01) as u16 & 0x8000 != 0, // left
            GetAsyncKeyState(0x02) as u16 & 0x8000 != 0, // right
            GetAsyncKeyState(0x04) as u16 & 0x8000 != 0, // middle
            GetAsyncKeyState(0x05) as u16 & 0x8000 != 0, // side 1
            GetAsyncKeyState(0x06) as u16 & 0x8000 != 0, // side 2
        ];
        for (i, down) in now.iter().enumerate() {
            if *down != last[i] {
                nany::input::on_mouse_button(i as u8, *down);
                last[i] = *down;
            }
        }
        drop(last);

        egui::Window::new("menu").show(f.ctx, |ui| {
            ui.label("hello from the overlay");
        });

        // return `true` when the ui wants the mouse and keyboard for itself (the framework swallows those window messages)
        // return `false` to let the input go to the game.
        true
    }
}
```

```rust
const DLL_PROCESS_ATTACH: u32 = 1;

#[no_mangle]
pub unsafe extern "system" fn DllMain(_h: *mut std::ffi::c_void, reason: u32, _r: *mut std::ffi::c_void) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        let hwnd = unsafe {
            windows::Win32::UI::WindowsAndMessaging::FindWindowA(
                windows::core::PCSTR::from_raw(b"GameWindowClass\0".as_ptr()),
                windows::core::PCSTR::null(),
            )
            .unwrap_or_default()
        };
        let menu = Box::new(MyMenu { hwnd, last: Mutex::new([false; 5]) });
        nany::on_dll_attach_tagged("my_overlay", menu);
    }
    1
}
```

```bash
# debug builds write `<dll>.log` next to the DLL; release builds write no log
cargo build --release

# cargo clippy --all-targets -- -D warnings
# cargo test
```

## Reference

- https://platform.deepseek.com
- https://github.com/veeenu/hudhook
- https://github.com/SaschaWillems/Vulkan
- https://github.com/ocornut/imgui