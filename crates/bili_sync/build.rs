use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // 首先生成built.rs文件 (用于版本信息等)
    built::write_built_file().expect("Failed to acquire build-time information");

    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR").unwrap();
    let target = env::var("TARGET").unwrap();

    // 确定目标平台的二进制文件名
    let binary_name = if target.contains("windows") {
        "aria2c.exe"
    } else {
        "aria2c"
    };

    let binary_path = Path::new(&out_dir).join(binary_name);

    // 如果二进制文件已存在，跳过获取
    if binary_path.exists() {
        println!("cargo:warning=aria2二进制文件已存在: {}", binary_path.display());
        return;
    }

    // 检查是否强制禁用下载（通过环境变量 BILI_SYNC_DOWNLOAD_ARIA2=false）
    let force_disable = env::var("BILI_SYNC_DOWNLOAD_ARIA2").unwrap_or_default() == "false";

    if force_disable {
        println!("cargo:warning=环境变量设置禁用下载，创建占位文件");
        handle_download_failure(&binary_path);
    } else {
        // 默认尝试获取aria2二进制文件
        println!("cargo:warning=尝试获取aria2二进制文件");
        if let Err(e) = get_aria2_for_ci(&target, &out_dir, binary_name) {
            println!("cargo:warning=获取aria2失败: {}", e);
            handle_download_failure(&binary_path);
        }
    }
}

fn get_aria2_for_ci(target: &str, out_dir: &str, binary_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if target.contains("windows") {
        // Windows: 下载预编译版本
        let url = "https://github.com/aria2/aria2/releases/download/release-1.37.0/aria2-1.37.0-win-64bit-build1.zip";
        let archive_name = "aria2-1.37.0-win-64bit-build1.zip";
        download_and_extract_windows(url, archive_name, out_dir, binary_name)
    } else if target.contains("linux") {
        // Linux: 下载静态链接版本
        download_static_aria2_linux(target, out_dir, binary_name)
    } else if target.contains("darwin") {
        // macOS: 下载预编译版本
        download_aria2_macos(out_dir, binary_name)
    } else {
        // 其他平台: 回退到系统安装方式
        install_aria2_from_system(out_dir, binary_name)
    }
}

fn download_static_aria2_linux(target: &str, out_dir: &str, binary_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let binary_path = Path::new(out_dir).join(binary_name);
    
    // 使用静态链接的aria2 builds from johang/aria2-static-builds
    let url = if target.contains("x86_64") {
        "https://github.com/johang/aria2-static-builds/releases/download/1.37.0/aria2-x86_64-linux"
    } else if target.contains("aarch64") {
        "https://github.com/johang/aria2-static-builds/releases/download/1.37.0/aria2-arm64-linux"
    } else {
        println!("cargo:warning=不支持的Linux架构: {}", target);
        return Err("不支持的Linux架构".into());
    };
    
    println!("cargo:warning=下载静态链接的aria2: {}", url);
    
    // 下载aria2二进制文件
    #[cfg(target_os = "windows")]
    {
        download_with_powershell(url, &binary_path)?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        download_with_curl_or_wget(url, &binary_path)?;
    }
    
    if !binary_path.exists() {
        return Err("下载的aria2文件不存在".into());
    }
    
    // 设置可执行权限
    set_executable_permissions(&binary_path)?;
    
    // 验证下载的文件
    if let Ok(output) = Command::new(&binary_path).arg("--version").output() {
        if output.status.success() {
            let version_info = String::from_utf8_lossy(&output.stdout);
            println!("cargo:warning=成功下载静态aria2: {}", version_info.lines().next().unwrap_or("unknown"));
            return Ok(());
        }
    }
    
    println!("cargo:warning=下载的aria2验证失败，回退到系统安装");
    install_aria2_from_system(out_dir, binary_name)
}

fn download_aria2_macos(out_dir: &str, binary_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:warning=macOS暂时使用系统安装方式");
    install_aria2_from_system(out_dir, binary_name)
}

fn install_aria2_from_system(out_dir: &str, binary_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let binary_path = Path::new(out_dir).join(binary_name);
    
    println!("cargo:warning=尝试从系统包管理器安装aria2");
    
    // 检查aria2是否已经安装
    if let Ok(output) = Command::new("which").arg("aria2c").output() {
        if output.status.success() {
            let system_path_raw = String::from_utf8_lossy(&output.stdout);
            let system_path = system_path_raw.trim();
            println!("cargo:warning=找到系统aria2: {}", system_path);
            
            // 复制系统的aria2c到我们的输出目录
            if let Err(e) = fs::copy(system_path, &binary_path) {
                println!("cargo:warning=复制系统aria2失败: {}", e);
                
                // 如果复制失败，尝试创建符号链接或硬链接
                #[cfg(unix)]
                if std::os::unix::fs::symlink(system_path, &binary_path).is_err() {
                    // 如果符号链接也失败，创建一个简单的包装脚本
                    let wrapper_script = format!("#!/bin/bash\nexec {} \"$@\"\n", system_path);
                    fs::write(&binary_path, wrapper_script)?;
                }
            }
            
            // 设置可执行权限
            set_executable_permissions(&binary_path)?;
            
            if binary_path.exists() {
                println!("cargo:warning=成功设置aria2二进制文件: {}", binary_path.display());
                return Ok(());
            }
        }
    }
    
    println!("cargo:warning=系统未安装aria2，尝试安装...");
    
    // 检测是否在容器中（通常cross编译时没有sudo）
    let use_sudo = Command::new("sudo").arg("--version").output().is_ok();
    
    // 检测操作系统并安装aria2
    let install_result = if cfg!(target_os = "linux") {
        // 尝试不同的Linux包管理器
        if Command::new("apt-get").arg("--version").output().is_ok() {
            println!("cargo:warning=使用apt-get安装aria2");
            if use_sudo {
                Command::new("sudo").args(["apt-get", "update", "-y"]).status().ok();
                Command::new("sudo").args(["apt-get", "install", "-y", "aria2"]).status()
            } else {
                println!("cargo:warning=容器环境，直接使用apt-get");
                Command::new("apt-get").args(["update", "-y"]).status().ok();
                Command::new("apt-get").args(["install", "-y", "aria2"]).status()
            }
        } else if Command::new("yum").arg("--version").output().is_ok() {
            println!("cargo:warning=使用yum安装aria2");
            if use_sudo {
                Command::new("sudo").args(["yum", "install", "-y", "aria2"]).status()
            } else {
                Command::new("yum").args(["install", "-y", "aria2"]).status()
            }
        } else if Command::new("dnf").arg("--version").output().is_ok() {
            println!("cargo:warning=使用dnf安装aria2");
            if use_sudo {
                Command::new("sudo").args(["dnf", "install", "-y", "aria2"]).status()
            } else {
                Command::new("dnf").args(["install", "-y", "aria2"]).status()
            }
        } else if Command::new("pacman").arg("--version").output().is_ok() {
            println!("cargo:warning=使用pacman安装aria2");
            if use_sudo {
                Command::new("sudo").args(["pacman", "-S", "--noconfirm", "aria2"]).status()
            } else {
                Command::new("pacman").args(["-S", "--noconfirm", "aria2"]).status()
            }
        } else {
            println!("cargo:warning=未找到支持的包管理器");
            return Err("未找到支持的包管理器".into());
        }
    } else if cfg!(target_os = "macos") {
        // macOS使用homebrew
        if Command::new("brew").arg("--version").output().is_ok() {
            println!("cargo:warning=使用homebrew安装aria2");
            Command::new("brew").args(["install", "aria2"]).status()
        } else {
            println!("cargo:warning=macOS未安装homebrew");
            return Err("macOS未安装homebrew".into());
        }
    } else {
        println!("cargo:warning=不支持的操作系统");
        return Err("不支持的操作系统".into());
    };
    
    match install_result {
        Ok(status) if status.success() => {
            println!("cargo:warning=aria2安装成功");
            
            // 再次尝试复制已安装的aria2
            if let Ok(output) = Command::new("which").arg("aria2c").output() {
                if output.status.success() {
                    let system_path_raw = String::from_utf8_lossy(&output.stdout);
                    let system_path = system_path_raw.trim();
                    println!("cargo:warning=找到新安装的aria2: {}", system_path);
                    
                    // 复制到我们的输出目录
                    if fs::copy(system_path, &binary_path).is_ok() {
                        set_executable_permissions(&binary_path)?;
                        
                        if binary_path.exists() {
                            println!("cargo:warning=成功复制aria2二进制文件: {}", binary_path.display());
                            return Ok(());
                        }
                    }
                }
            }
            
            Err("安装成功但找不到aria2二进制文件".into())
        }
        Ok(_) => Err("aria2安装失败".into()),
        Err(e) => Err(format!("安装aria2时出错: {}", e).into()),
    }
}

fn download_and_extract_windows(
    url: &str,
    archive_name: &str,
    out_dir: &str,
    binary_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let archive_path = Path::new(out_dir).join(archive_name);
    let binary_path = Path::new(out_dir).join(binary_name);

    // 下载文件
    println!("cargo:warning=下载 {} 到 {}", url, archive_path.display());
    download_with_powershell(url, &archive_path)?;

    if !archive_path.exists() {
        return Err("下载的文件不存在".into());
    }

    // 解压文件
    println!("cargo:warning=解压 {} 到 {}", archive_path.display(), out_dir);
    extract_zip(&archive_path, out_dir, binary_name)?;

    // 删除下载的压缩包
    let _ = fs::remove_file(&archive_path);

    if binary_path.exists() {
        println!("cargo:warning=成功提取aria2二进制文件: {}", binary_path.display());
        Ok(())
    } else {
        Err("提取后的二进制文件不存在".into())
    }
}

fn download_with_curl_or_wget(url: &str, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // 首先尝试curl
    if Command::new("curl").arg("--version").output().is_ok() {
        println!("cargo:warning=使用curl下载: {}", url);
        let status = Command::new("curl")
            .args(["-L", "-o"])
            .arg(output)
            .arg(url)
            .status()?;
        
        if status.success() {
            return Ok(());
        }
    }
    
    // 如果curl失败，尝试wget
    if Command::new("wget").arg("--version").output().is_ok() {
        println!("cargo:warning=使用wget下载: {}", url);
        let status = Command::new("wget")
            .args(["-O"])
            .arg(output)
            .arg(url)
            .status()?;
        
        if status.success() {
            return Ok(());
        }
    }
    
    Err("curl和wget都不可用或下载失败".into())
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

fn extract_zip(archive_path: &Path, out_dir: &str, binary_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 在Windows上使用PowerShell解压
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

    Ok(())
}

fn handle_download_failure(binary_path: &Path) {
    println!("cargo:warning=创建aria2占位文件，运行时需要系统安装aria2");

    // 创建父目录
    if let Some(parent) = binary_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // 创建占位文件
    let content: Vec<u8> = if binary_path.extension().unwrap_or_default() == "exe" {
        // Windows可执行文件
        b"echo Please install aria2 manually && pause".to_vec()
    } else {
        // Unix可执行脚本
        b"#!/bin/bash\necho 'Please install aria2 manually (apt install aria2 / brew install aria2)'\nread -p 'Press Enter to continue...'".to_vec()
    };

    if fs::write(binary_path, content).is_ok() {
        let _ = set_executable_permissions(binary_path);
        println!("cargo:warning=已创建占位文件: {}", binary_path.display());
    }
}

#[cfg(unix)]
fn set_executable_permissions(binary_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(binary_path)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(binary_path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable_permissions(_binary_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
