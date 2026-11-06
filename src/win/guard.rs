use super::{ocr, screen, system, Shared};
use crate::actions::{parse_combo, Rect};
use crate::guard::{self, Level, Verdict};
use base64::Engine;
use std::collections::HashMap;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::core::HSTRING;
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapEncoder, BitmapPixelFormat};
use windows::Storage::Streams::{DataReader, InMemoryRandomAccessStream};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MB_SETFOREGROUND, MB_SYSTEMMODAL, MB_TOPMOST};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const HOLD: Duration = Duration::from_secs(30 * 60);
const RISKY_TITLES: &[&str] = &["bank", "pay", "upi", "otp", "kyc", "alert", "support", "virus", "warning", "infected", "arrest", "police", "cbi", "customs", "gift", "refund", "lottery", "uninstall", "call", "video"];

fn processes() -> Vec<String> {
    let out = Command::new("tasklist.exe").args(["/fo", "csv", "/nh"]).creation_flags(CREATE_NO_WINDOW).stderr(Stdio::null()).output();
    let Ok(out) = out else { return Vec::new() };
    String::from_utf8_lossy(&out.stdout).lines().filter_map(|l| l.split("\",\"").next().map(|n| n.trim_matches('"').to_string())).filter(|n| !n.is_empty()).collect()
}

fn kill(image: &str) -> bool {
    Command::new("taskkill.exe").args(["/F", "/T", "/IM", image]).creation_flags(CREATE_NO_WINDOW).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

fn jpeg(bgra: &[u8], w: u32, h: u32) -> Result<Vec<u8>, String> {
    let e = |x: windows::core::Error| x.to_string();
    let stream = InMemoryRandomAccessStream::new().map_err(e)?;
    let encoder = BitmapEncoder::CreateAsync(BitmapEncoder::JpegEncoderId().map_err(e)?, &stream).map_err(e)?.get().map_err(e)?;
    encoder.SetPixelData(BitmapPixelFormat::Bgra8, BitmapAlphaMode::Ignore, w, h, 96.0, 96.0, bgra).map_err(e)?;
    encoder.FlushAsync().map_err(e)?.get().map_err(e)?;
    let size = u32::try_from(stream.Size().map_err(e)?).map_err(|_| "screenshot too large".to_string())?;
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0).map_err(e)?).map_err(e)?;
    reader.LoadAsync(size).map_err(e)?.get().map_err(e)?;
    let mut out = vec![0u8; size as usize];
    reader.ReadBytes(&mut out).map_err(e)?;
    Ok(out)
}

struct Look {
    rect: Rect,
    pixels: Vec<u8>,
    lines: Vec<ocr::Line>,
}

fn look(reader: Option<&ocr::Reader>) -> Option<Look> {
    let rect = screen::monitor_of(system::target_window());
    let pixels = screen::grab(rect)?;
    let lines = reader.and_then(|r| r.read(&pixels, rect.width() as u32, rect.height() as u32, (rect.left, rect.top)).ok()).unwrap_or_default();
    Some(Look { rect, pixels, lines })
}

fn redacted_jpeg(l: &Look) -> Option<String> {
    let (w, h) = (l.rect.width(), l.rect.height());
    let mut px = l.pixels.clone();
    for line in l.lines.iter().filter(|line| guard::should_redact(&line.text)) {
        let (x0, y0) = ((line.rect.left - l.rect.left - 4).max(0), (line.rect.top - l.rect.top - 3).max(0));
        let (x1, y1) = ((line.rect.right - l.rect.left + 4).min(w), (line.rect.bottom - l.rect.top + 3).min(h));
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * w + x) * 4) as usize;
                px[i..i + 4].copy_from_slice(&[0, 0, 0, 255]);
            }
        }
    }
    let scale = (1280.0 / f64::from(w.max(1))).min(1.0);
    let (small, sw, sh) = if scale < 1.0 { ocr::shrink(&px, w as u32, h as u32, scale) } else { (px, w as u32, h as u32) };
    let bytes = jpeg(&small, sw, sh).ok()?;
    if bytes.len() > 640_000 {
        return None;
    }
    Some(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

fn pause(text: String, stop: bool) {
    std::thread::spawn(move || {
        let body = if stop {
            format!("{text}\n\nYour family has been told and can see what happened.\nTo talk to someone now, call your family, or 1930 (cyber fraud helpline).")
        } else {
            format!("{text}\n\nYour family has been told.")
        };
        let icon = if stop { MB_ICONWARNING } else { MB_ICONINFORMATION };
        unsafe {
            let _ = MessageBoxW(HWND::default(), &HSTRING::from(body), &HSTRING::from(if stop { "Sahay stopped a scam" } else { "Sahay: please wait" }), MB_OK | icon | MB_SYSTEMMODAL | MB_TOPMOST | MB_SETFOREGROUND);
        }
    });
}

fn act(v: &Verdict, procs: &[String], titles: &[(isize, String)]) -> Vec<String> {
    let mut done = Vec::new();
    for key in &v.kill {
        for image in procs.iter().filter(|p| p.to_lowercase().replace(".exe", "").contains(key.as_str())) {
            if kill(image) {
                done.push(format!("Closed {image}"));
            }
        }
    }
    if v.close_page {
        let browser = titles.iter().find(|(_, t)| {
            let t = t.to_lowercase();
            ["chrome", "edge", "firefox", "brave", "opera"].iter().any(|b| t.contains(b))
        });
        if let Some((_, title)) = browser {
            if system::focus(title).is_ok() {
                std::thread::sleep(Duration::from_millis(250));
                if parse_combo("ctrl+w").and_then(|k| super::input::combo(&k, 1)).is_ok() {
                    done.push(format!("Closed the page \"{}\"", title.chars().take(60).collect::<String>()));
                }
            }
        }
    }
    done.dedup();
    done
}

pub fn run(shared: Arc<Shared>) {
    let port = std::fs::read_to_string(system::home().join("saathi.env")).map(|t| crate::saathi::port_from_env_file(&t)).unwrap_or(8787);
    let reader = ocr::Reader::new().ok();
    let mut last_seen: HashMap<&'static str, Instant> = HashMap::new();
    let mut hold: Option<(Instant, Option<String>, Instant)> = None;
    let mut last_ocr = Instant::now() - Duration::from_secs(60);
    loop {
        for _ in 0..8 {
            if shared.quit.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        let procs = processes();
        let windows = system::user_windows();
        let titles: Vec<String> = windows.iter().map(|(_, t)| t.clone()).collect();

        if let Some((since, id, checked)) = hold.as_mut() {
            if since.elapsed() > HOLD {
                hold = None;
            } else {
                if checked.elapsed() > Duration::from_secs(30) {
                    *checked = Instant::now();
                    if let Some(id) = id.as_deref() {
                        if let Ok(false) = crate::tasks::hold_active(port, id) {
                            system::log("scam guard: family unblocked");
                            hold = None;
                        }
                    }
                }
                if hold.is_some() {
                    for (exe, name) in guard::REMOTE_TOOLS {
                        for image in procs.iter().filter(|p| p.to_lowercase().contains(exe)) {
                            if kill(image) {
                                shared.reply(&format!("I closed {name} again. Your family asked to keep remote access blocked for now."));
                            }
                        }
                    }
                    continue;
                }
            }
        }

        let lower_titles = titles.join(" ").to_lowercase();
        let risky = guard::REMOTE_TOOLS.iter().any(|(exe, _)| procs.iter().any(|p| p.to_lowercase().contains(exe))) || RISKY_TITLES.iter().any(|w| lower_titles.contains(w));
        let mut snapshot = None;
        let text = if risky && last_ocr.elapsed() > Duration::from_secs(4) {
            last_ocr = Instant::now();
            snapshot = look(reader.as_ref());
            snapshot.as_ref().map(|l| l.lines.iter().map(|x| x.text.as_str()).collect::<Vec<_>>().join(" ")).unwrap_or_default()
        } else {
            String::new()
        };
        let verdict = guard::assess(&procs, &titles, &text);
        if verdict.level == Level::Calm {
            continue;
        }
        if last_seen.get(verdict.kind).is_some_and(|t| t.elapsed() < Duration::from_secs(300)) {
            continue;
        }
        last_seen.insert(verdict.kind, Instant::now());
        system::log(&format!("scam guard: {} ({})", verdict.kind, verdict.why.join("; ")));
        let snapshot = snapshot.or_else(|| look(reader.as_ref()));
        let screenshot = snapshot.as_ref().and_then(redacted_jpeg);
        let stop = verdict.level == Level::Stop;
        if stop {
            shared.stop.store(true, Ordering::Relaxed);
        }
        let actions = if stop { act(&verdict, &procs, &windows) } else { Vec::new() };
        let (explain, _) = guard::explain(verdict.kind);
        shared.set_status(super::Status::Waiting, "Safety pause");
        shared.reply(explain);
        pause(explain.to_string(), stop);
        let mut report_actions = actions.clone();
        if stop {
            report_actions.push("Paused the computer and explained the scam".into());
        }
        let id = crate::tasks::report_incident(port, &guard::incident(&verdict, &report_actions, screenshot)).ok();
        if stop && verdict.kind != "tech-support" && verdict.kind != "otp-page" {
            hold = Some((Instant::now(), id, Instant::now()));
        }
        shared.set_status(super::Status::Idle, "");
    }
}
