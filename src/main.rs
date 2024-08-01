use dirs::picture_dir;
use dotenv::dotenv;
use reqwest::blocking::get;
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::copy;
use std::path::PathBuf;

// Struct to deserialize the API response
#[derive(Debug, Deserialize)]
struct ApiResponse {
    hdurl: String,
}

fn fetch_image_data(api_url: &str) -> Result<ApiResponse, Box<dyn std::error::Error>> {
    let response = get(api_url)?;
    let api_response: ApiResponse = response.json()?;
    Ok(api_response)
}

fn download_image(url: &str, filename: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let response = get(url)?;
    let mut out = File::create(filename)?;
    let mut content = std::io::Cursor::new(response.bytes()?);
    copy(&mut content, &mut out)?;
    Ok(PathBuf::from(filename))
}

#[cfg(target_os = "windows")]
mod windows_background {
    use std::ffi::OsStr;
    use std::path::Path;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW;
    use windows::Win32::UI::WindowsAndMessaging::SPI_SETDESKWALLPAPER;

    pub fn set_wallpaper(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let path_str: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
        unsafe {
            let _ = SystemParametersInfoW(
                SPI_SETDESKWALLPAPER,
                0,
                Some(path_str.as_ptr() as *mut _),
                windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod linux_background {
    use std::env;
    use std::path::Path;
    use std::process::Command;

    pub fn set_wallpaper(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let desktop_env = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        match desktop_env.as_str() {
            // Handle GNOME desktops
            env if env.contains("GNOME") => {
                let output = Command::new("gsettings")
                    .args(&[
                        "set",
                        "org.gnome.desktop.background",
                        "picture-uri",
                        &format!("file://{}", path.display()),
                    ])
                    .output()?;

                if !output.status.success() {
                    return Err(format!(
                        "Failed to set GNOME wallpaper: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )
                    .into());
                }
            }
            // Handle KDE desktops
            env if env.contains("KDE") => {
                let script = format!(
                    r#"
                var allDesktops = desktops();
                    d = allDesktops[0];
                    d.wallpaperPlugin = "org.kde.image";
                    d.currentConfigGroup = Array("Wallpaper", "org.kde.image", "General");
                    d.writeConfig("Image", "file://{}")
                "#,
                    path.display()
                );

                let output = Command::new("qdbus")
                    .args(&[
                        "org.kde.plasmashell",
                        "/PlasmaShell",
                        "org.kde.PlasmaShell.evaluateScript",
                        &script,
                    ])
                    .output()?;

                if !output.status.success() {
                    return Err(format!(
                        "Failed to set KDE wallpaper: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )
                    .into());
                }
            }
            _ => {
                return Err("Unsupported desktop environment".into());
            }
        }
        Ok(())
    }
}

fn main() {
    dotenv().ok();

    // Get API URL from environment variable
    let api_key = env::var("APOD_KEY").expect("APOD_KEY must be set in the environment");
    let api_url = format!("https://api.nasa.gov/planetary/apod?api_key={api_key}");

    match fetch_image_data(&api_url) {
        Ok(api_response) => {
            println!("Fetched image data: {:?}", api_response);

            let filename = "apod.jpg";
            let download_path = picture_dir()
                .expect("Could not find picture directory")
                .join(filename);

            match download_image(&api_response.hdurl, download_path.to_str().unwrap()) {
                Ok(path) => {
                    println!("Image downloaded to {:?}", path);

                    #[cfg(target_os = "windows")]
                    {
                        if let Err(e) = windows_background::set_wallpaper(&path) {
                            eprintln!("Failed to set wallpaper on Windows: {}", e);
                        }
                    }

                    #[cfg(target_os = "linux")]
                    {
                        if let Err(e) = linux_background::set_wallpaper(&path) {
                            eprintln!("Failed to set wallpaper on Linux: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to download image: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch image data: {}", e);
        }
    }
}
