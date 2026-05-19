use std::path::Path;
use std::process::{Command, Stdio};

const SOUND_PATHS: [&str; 3] = [
    "/usr/share/sounds/freedesktop/stereo/bell.oga",
    "/usr/share/sounds/freedesktop/stereo/message.oga",
    "/usr/share/sounds/freedesktop/stereo/water-drop.oga",
];

pub fn play_sound() {
    for path in SOUND_PATHS {
        if !Path::new(path).exists() {
            continue;
        }
        let spawned = Command::new("paplay")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if spawned.is_ok() {
            return;
        }
    }
}
