use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // 首先生成built.rs文件 (用于版本信息等)
    built::write_built_file().expect("Failed to acquire build-time information");

    println!("cargo:rerun-if-changed=build.rs");

    // 检查是否在CI环境中
    let is_ci =
        env::var("CI").unwrap_or_default() == "true" || env::var("GITHUB_ACTIONS").unwrap_or_default() == "true";

    let out_dir = env::var("OUT_DIR").unwrap();
    let target = env::var("TARGET").unwrap();

    // 确定aria2下载信息
    let (download_url, archive_name, binary_name) = get_aria2_info(&target);

    let binary_path = Path::new(&out_dir).join(binary_name);

    // 如果二进制文件已存在，跳过下载
    if binary_path.exists() {
        println!("cargo:warning=aria2二进制文件已存在: {}", binary_path.display());
        return;
    }

    if is_ci {
        println!("cargo:warning=检测到CI环境，尝试下载aria2二进制文件");

        // 在CI环境中尝试下载
        if let Err(e) = download_and_extract(download_url, archive_name, &out_dir, binary_name) {
            println!("cargo:warning=下载失败: {}", e);
            handle_download_failure(&binary_path);
        }
    } else {
        // 检查是否强制下载（通过环境变量 BILI_SYNC_DOWNLOAD_ARIA2=true）
        let force_download = env::var("BILI_SYNC_DOWNLOAD_ARIA2").unwrap_or_default() == "true";
        // 检查是否强制禁用下载（通过环境变量 BILI_SYNC_DOWNLOAD_ARIA2=false）
        let force_disable = env::var("BILI_SYNC_DOWNLOAD_ARIA2").unwrap_or_default() == "false";

        if force_disable {
            println!("cargo:warning=环境变量设置禁用下载，创建占位文件");
            handle_download_failure(&binary_path);
        } else if force_download {
            println!("cargo:warning=环境变量设置强制下载aria2二进制文件");
            if let Err(e) = download_and_extract(download_url, archive_name, &out_dir, binary_name) {
                println!("cargo:warning=下载失败: {}", e);
                handle_download_failure(&binary_path);
            }
        } else {
            // 本地环境直接尝试下载，失败则回退到占位文件
            println!("cargo:warning=本地环境，尝试下载aria2二进制文件...");
            if let Err(e) = download_and_extract(download_url, archive_name, &out_dir, binary_name) {
                println!("cargo:warning=下载失败，回退到占位文件: {}", e);
                handle_download_failure(&binary_path);
            }
        }
    }
}

fn get_aria2_info(target: &str) -> (&'static str, &'static str, &'static str) {
    match target {
        t if t.contains("windows") => (
            "https://github.com/aria2/aria2/releases/download/release-1.37.0/aria2-1.37.0-win-64bit-build1.zip",
            "aria2-1.37.0-win-64bit-build1.zip",
            "aria2c.exe"
        ),
        t if t.contains("linux") && t.contains("musl") => (
            "https://github.com/aria2/aria2/releases/download/release-1.37.0/aria2-1.37.0-linux-gnu-64bit-build1.tar.bz2",
            "aria2-1.37.0-linux-gnu-64bit-build1.tar.bz2",
            "aria2c"
        ),
        t if t.contains("apple") => (
            "https://github.com/aria2/aria2/releases/download/release-1.37.0/aria2-1.37.0-osx-darwin.dmg",
            "aria2-1.37.0-osx-darwin.dmg",
            "aria2c"
        ),
        _ => (
            "https://github.com/aria2/aria2/releases/download/release-1.37.0/aria2-1.37.0-linux-gnu-64bit-build1.tar.bz2",
            "aria2-1.37.0-linux-gnu-64bit-build1.tar.bz2",
            "aria2c"
        ),
    }
}

fn download_and_extract(
    url: &str,
    archive_name: &str,
    out_dir: &str,
    binary_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let archive_path = Path::new(out_dir).join(archive_name);
    let binary_path = Path::new(out_dir).join(binary_name);

    // 下载文件
    println!("cargo:warning=下载 {} 到 {}", url, archive_path.display());

    if cfg!(target_os = "windows") {
        download_with_powershell(url, &archive_path)?;
    } else {
        download_with_curl_or_wget(url, &archive_path)?;
    }

    if !archive_path.exists() {
        return Err("下载的文件不存在".into());
    }

    // 解压文件
    println!("cargo:warning=解压 {} 到 {}", archive_path.display(), out_dir);

    if archive_name.ends_with(".zip") {
        extract_zip(&archive_path, out_dir, binary_name)?;
    } else if archive_name.ends_with(".tar.bz2") {
        extract_tar_bz2(&archive_path, out_dir, binary_name)?;
    }

    // 删除下载的压缩包
    let _ = fs::remove_file(&archive_path);

    // 在Unix系统上设置可执行权限
    if !cfg!(target_os = "windows") {
        set_executable_permissions(&binary_path)?;
    }

    if binary_path.exists() {
        println!("cargo:warning=成功提取aria2二进制文件: {}", binary_path.display());
    } else {
        return Err("提取后的二进制文件不存在".into());
    }

    Ok(())
}

fn download_with_powershell(url: &str, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("powershell")
        .args([
            "-Command",
            &format!("Invoke-WebRequest -Uri '{}' -OutFile '{}'", url, output.display()),
        ])
        .status()?;

    if !status.success() {
        return Err("PowerShell下载失败".into());
    }

    Ok(())
}

fn download_with_curl_or_wget(url: &str, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // 首先尝试curl
    let curl_status = Command::new("curl")
        .args(["-L", "-o", &output.to_string_lossy(), url])
        .status();

    if let Ok(status) = curl_status {
        if status.success() {
            return Ok(());
        }
    }

    // 如果curl失败，尝试wget
    let wget_status = Command::new("wget")
        .args(["-O", &output.to_string_lossy(), url])
        .status()?;

    if !wget_status.success() {
        return Err("curl和wget都下载失败".into());
    }

    Ok(())
}

fn extract_zip(archive_path: &Path, out_dir: &str, binary_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 在Windows上使用PowerShell解压
    if cfg!(target_os = "windows") {
        let extract_script = format!(
            "Add-Type -AssemblyName System.IO.Compression.FileSystem; \
             $zip = [System.IO.Compression.ZipFile]::OpenRead('{}'); \
             $entry = $zip.Entries | Where-Object {{ $_.Name -eq '{}' }}; \
             if ($entry) {{ \
                 [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, '{}', $true); \
             }}; \
             $zip.Dispose()",
            archive_path.display(),
            binary_name,
            Path::new(out_dir).join(binary_name).display()
        );

        let status = Command::new("powershell")
            .args(["-Command", &extract_script])
            .status()?;

        if !status.success() {
            return Err("PowerShell解压失败".into());
        }
    } else {
        // 在Unix系统上使用unzip
        let status = Command::new("unzip")
            .args(["-j", &archive_path.to_string_lossy(), binary_name, "-d", out_dir])
            .status()?;

        if !status.success() {
            return Err("unzip解压失败".into());
        }
    }

    Ok(())
}

fn extract_tar_bz2(archive_path: &Path, out_dir: &str, binary_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("tar")
        .args([
            "-xjf",
            &archive_path.to_string_lossy(),
            "-C",
            out_dir,
            "--strip-components=1",
            &format!("*/bin/{}", binary_name),
        ])
        .status()?;

    if !status.success() {
        return Err("tar解压失败".into());
    }

    Ok(())
}

fn handle_download_failure(binary_path: &Path) {
    println!("cargo:warning=无法下载aria2，创建占位文件: {}", binary_path.display());
    println!("cargo:warning=提示：可设置环境变量控制下载行为：");
    println!("cargo:warning=  BILI_SYNC_DOWNLOAD_ARIA2=true  强制下载");
    println!("cargo:warning=  BILI_SYNC_DOWNLOAD_ARIA2=false 禁用下载");

    // 创建一个简单的占位文件
    let placeholder_content = b"placeholder";
    if let Err(e) = fs::write(binary_path, placeholder_content) {
        println!("cargo:warning=创建占位文件失败: {}", e);
    }
}

#[cfg(unix)]
fn set_executable_permissions(binary_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(binary_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(binary_path, perms)?;

    Ok(())
}

#[cfg(not(unix))]
fn set_executable_permissions(_binary_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
