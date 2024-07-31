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
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows::Win32::Foundation::PWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_SETDESKWALLPAPER};

    pub fn set_wallpaper(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let path_str: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            SystemParametersInfoW(
                SPI_SETDESKWALLPAPER,
                0,
                PWSTR(path_str.as_ptr() as *mut _),
                0,
            );
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod linux_background {
    use std::path::Path;
    use std::process::Command;

    pub fn set_wallpaper(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        Command::new("gsettings")
            .args(&[
                "set",
                "org.gnome.desktop.background",
                "picture-uri",
                &format!("file://{}", path.display()),
            ])
            .output()?;
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

            let filename = "background.jpg";
            let download_path = picture_dir().unwrap().join(filename);

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
