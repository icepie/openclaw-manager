use crate::utils::{platform, shell};
use serde::{Deserialize, Serialize};
use tauri::{command, Emitter, Manager};
use log::{info, warn, error, debug};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// 全局取消标志
static INSTALL_CANCELLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 环境检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentStatus {
    /// Git 是否安装
    pub git_installed: bool,
    /// Git 版本
    pub git_version: Option<String>,
    /// Node.js 是否安装
    pub node_installed: bool,
    /// Node.js 版本
    pub node_version: Option<String>,
    /// Node.js 版本是否满足要求 (>=22)
    pub node_version_ok: bool,
    /// OpenClaw 是否安装
    pub openclaw_installed: bool,
    /// OpenClaw 版本
    pub openclaw_version: Option<String>,
    /// 配置目录是否存在
    pub config_dir_exists: bool,
    /// 是否全部就绪
    pub ready: bool,
    /// 操作系统
    pub os: String,
}

/// 安装进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub step: String,
    pub progress: u8,
    pub message: String,
    pub error: Option<String>,
}

/// 安装结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub message: String,
    pub error: Option<String>,
}

/// 检查环境状态
#[command]
pub async fn check_environment() -> Result<EnvironmentStatus, String> {
    info!("[环境检查] 开始检查系统环境...");
    
    let os = platform::get_os();
    info!("[环境检查] 操作系统: {}", os);

    // 检查 Git
    info!("[环境检查] 检查 Git...");
    let git_version = shell::run_command_output("git", &["--version"]).ok()
        .map(|v| v.trim().to_string());
    let git_installed = git_version.is_some();
    info!("[环境检查] Git: installed={}, version={:?}", git_installed, git_version);

    // 检查 Node.js
    info!("[环境检查] 检查 Node.js...");
    let node_version = get_node_version();
    let node_installed = node_version.is_some();
    let node_version_ok = check_node_version_requirement(&node_version);
    info!("[环境检查] Node.js: installed={}, version={:?}, version_ok={}", 
        node_installed, node_version, node_version_ok);
    
    // 检查 OpenClaw
    info!("[环境检查] 检查 OpenClaw...");
    let openclaw_version = get_openclaw_version();
    let openclaw_installed = openclaw_version.is_some();
    info!("[环境检查] OpenClaw: installed={}, version={:?}", 
        openclaw_installed, openclaw_version);
    
    // 检查配置目录
    let config_dir = platform::get_config_dir();
    let config_dir_exists = std::path::Path::new(&config_dir).exists();
    info!("[环境检查] 配置目录: {}, exists={}", config_dir, config_dir_exists);
    
    let ready = openclaw_installed;
    info!("[环境检查] 环境就绪状态: ready={}", ready);

    Ok(EnvironmentStatus {
        git_installed,
        git_version,
        node_installed,
        node_version,
        node_version_ok,
        openclaw_installed,
        openclaw_version,
        config_dir_exists,
        ready,
        os,
    })
}

/// 获取 Node.js 版本
/// 检测多个可能的安装路径，因为 GUI 应用不继承用户 shell 的 PATH
fn get_node_version() -> Option<String> {
    // 优先检查离线安装的 bundled node
    if let Some(home) = dirs::home_dir() {
        let bundled = if platform::is_windows() {
            home.join(".openclaw").join("node").join("node.exe")
        } else {
            home.join(".openclaw").join("node").join("node")
        };
        if bundled.exists() {
            let result = if platform::is_windows() {
                shell::run_cmd_output(&format!("\"{}\" --version", bundled.display()))
            } else {
                shell::run_command_output(&bundled.to_string_lossy(), &["--version"])
            };
            if let Ok(v) = result {
                let v = v.trim().to_string();
                if v.starts_with('v') {
                    info!("[环境检查] 找到 bundled Node.js: {}", v);
                    return Some(v);
                }
            }
        }
    }

    if platform::is_windows() {
        if let Ok(v) = shell::run_cmd_output("node --version") {
            let version = v.trim().to_string();
            if !version.is_empty() && version.starts_with('v') {
                return Some(version);
            }
        }
        let possible_paths = get_windows_node_paths();
        for path in possible_paths {
            if std::path::Path::new(&path).exists() {
                let cmd = format!("\"{}\" --version", path);
                if let Ok(output) = shell::run_cmd_output(&cmd) {
                    let version = output.trim().to_string();
                    if !version.is_empty() && version.starts_with('v') {
                        return Some(version);
                    }
                }
            }
        }
        None
    } else {
        if let Ok(v) = shell::run_command_output("node", &["--version"]) {
            return Some(v.trim().to_string());
        }
        let possible_paths = get_unix_node_paths();
        for path in possible_paths {
            if std::path::Path::new(&path).exists() {
                if let Ok(output) = shell::run_command_output(&path, &["--version"]) {
                    return Some(output.trim().to_string());
                }
            }
        }
        if let Ok(output) = shell::run_bash_output("source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null; node --version 2>/dev/null") {
            if !output.is_empty() && output.starts_with('v') {
                return Some(output.trim().to_string());
            }
        }
        None
    }
}

/// 获取 Unix 系统上可能的 Node.js 路径
fn get_unix_node_paths() -> Vec<String> {
    let mut paths = Vec::new();
    
    // Homebrew (macOS)
    paths.push("/opt/homebrew/bin/node".to_string()); // Apple Silicon
    paths.push("/usr/local/bin/node".to_string());     // Intel Mac
    
    // 系统安装
    paths.push("/usr/bin/node".to_string());
    
    // nvm (检查常见版本)
    if let Some(home) = dirs::home_dir() {
        let home_str = home.display().to_string();
        
        // nvm 默认版本
        paths.push(format!("{}/.nvm/versions/node/v22.0.0/bin/node", home_str));
        paths.push(format!("{}/.nvm/versions/node/v22.1.0/bin/node", home_str));
        paths.push(format!("{}/.nvm/versions/node/v22.2.0/bin/node", home_str));
        paths.push(format!("{}/.nvm/versions/node/v22.11.0/bin/node", home_str));
        paths.push(format!("{}/.nvm/versions/node/v22.12.0/bin/node", home_str));
        paths.push(format!("{}/.nvm/versions/node/v23.0.0/bin/node", home_str));
        
        // 尝试 nvm alias default（读取 nvm 的 default alias）
        let nvm_default = format!("{}/.nvm/alias/default", home_str);
        if let Ok(version) = std::fs::read_to_string(&nvm_default) {
            let version = version.trim();
            if !version.is_empty() {
                paths.insert(0, format!("{}/.nvm/versions/node/v{}/bin/node", home_str, version));
            }
        }
        
        // fnm
        paths.push(format!("{}/.fnm/aliases/default/bin/node", home_str));
        
        // volta
        paths.push(format!("{}/.volta/bin/node", home_str));
        
        // asdf
        paths.push(format!("{}/.asdf/shims/node", home_str));
        
        // mise (formerly rtx)
        paths.push(format!("{}/.local/share/mise/shims/node", home_str));
    }
    
    paths
}

/// 获取 Windows 系统上可能的 Node.js 路径
fn get_windows_node_paths() -> Vec<String> {
    let mut paths = Vec::new();
    
    // 1. 标准安装路径 (Program Files)
    paths.push("C:\\Program Files\\nodejs\\node.exe".to_string());
    paths.push("C:\\Program Files (x86)\\nodejs\\node.exe".to_string());
    
    // 2. nvm for Windows (nvm4w) - 常见安装位置
    paths.push("C:\\nvm4w\\nodejs\\node.exe".to_string());
    
    // 3. 用户目录下的各种安装
    if let Some(home) = dirs::home_dir() {
        let home_str = home.display().to_string();
        
        // nvm for Windows 用户安装
        paths.push(format!("{}\\AppData\\Roaming\\nvm\\current\\node.exe", home_str));
        
        // fnm (Fast Node Manager) for Windows
        paths.push(format!("{}\\AppData\\Roaming\\fnm\\aliases\\default\\node.exe", home_str));
        paths.push(format!("{}\\AppData\\Local\\fnm\\aliases\\default\\node.exe", home_str));
        paths.push(format!("{}\\.fnm\\aliases\\default\\node.exe", home_str));
        
        // volta
        paths.push(format!("{}\\AppData\\Local\\Volta\\bin\\node.exe", home_str));
        // volta 通过 shim 调用，检查 bin 目录即可
        
        // scoop 安装
        paths.push(format!("{}\\scoop\\apps\\nodejs\\current\\node.exe", home_str));
        paths.push(format!("{}\\scoop\\apps\\nodejs-lts\\current\\node.exe", home_str));
        
        // chocolatey 安装
        paths.push("C:\\ProgramData\\chocolatey\\lib\\nodejs\\tools\\node.exe".to_string());
    }
    
    // 4. 从注册表读取的安装路径（通过环境变量间接获取）
    if let Ok(program_files) = std::env::var("ProgramFiles") {
        paths.push(format!("{}\\nodejs\\node.exe", program_files));
    }
    if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
        paths.push(format!("{}\\nodejs\\node.exe", program_files_x86));
    }
    
    // 5. nvm-windows 的符号链接路径（NVM_SYMLINK 环境变量）
    if let Ok(nvm_symlink) = std::env::var("NVM_SYMLINK") {
        paths.insert(0, format!("{}\\node.exe", nvm_symlink));
    }
    
    // 6. nvm-windows 的 NVM_HOME 路径下的当前版本
    if let Ok(nvm_home) = std::env::var("NVM_HOME") {
        // 尝试读取当前激活的版本
        let settings_path = format!("{}\\settings.txt", nvm_home);
        if let Ok(content) = std::fs::read_to_string(&settings_path) {
            for line in content.lines() {
                if line.starts_with("current:") {
                    if let Some(version) = line.strip_prefix("current:") {
                        let version = version.trim();
                        if !version.is_empty() {
                            paths.insert(0, format!("{}\\v{}\\node.exe", nvm_home, version));
                        }
                    }
                }
            }
        }
    }
    
    paths
}

/// 获取 OpenClaw 版本
fn get_openclaw_version() -> Option<String> {
    // 使用 run_openclaw 统一处理各平台
    shell::run_openclaw(&["--version"])
        .ok()
        .map(|v| v.trim().to_string())
}

/// 检查 Node.js 版本是否 >= 22
fn check_node_version_requirement(version: &Option<String>) -> bool {
    if let Some(v) = version {
        // 解析版本号 "v22.1.0" -> 22
        let major = v.trim_start_matches('v')
            .split('.')
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        major >= 22
    } else {
        false
    }
}

/// 安装 Node.js
#[command]
pub async fn install_nodejs() -> Result<InstallResult, String> {
    info!("[安装Node.js] 开始安装 Node.js...");
    let os = platform::get_os();
    info!("[安装Node.js] 检测到操作系统: {}", os);
    
    let result = match os.as_str() {
        "windows" => {
            info!("[安装Node.js] 使用 Windows 安装方式...");
            install_nodejs_windows().await
        },
        "macos" => {
            info!("[安装Node.js] 使用 macOS 安装方式 (Homebrew)...");
            install_nodejs_macos().await
        },
        "linux" => {
            info!("[安装Node.js] 使用 Linux 安装方式...");
            install_nodejs_linux().await
        },
        _ => {
            error!("[安装Node.js] 不支持的操作系统: {}", os);
            Ok(InstallResult {
                success: false,
                message: "不支持的操作系统".to_string(),
                error: Some(format!("不支持的操作系统: {}", os)),
            })
        },
    };
    
    match &result {
        Ok(r) if r.success => info!("[安装Node.js] ✓ 安装成功"),
        Ok(r) => warn!("[安装Node.js] ✗ 安装失败: {}", r.message),
        Err(e) => error!("[安装Node.js] ✗ 安装错误: {}", e),
    }
    
    result
}

/// Windows 安装 Node.js
async fn install_nodejs_windows() -> Result<InstallResult, String> {
    // 使用 winget 安装 Node.js（Windows 10/11 自带）
    let script = r#"
$ErrorActionPreference = 'Stop'

# 检查是否已安装
$nodeVersion = node --version 2>$null
if ($nodeVersion) {
    Write-Host "Node.js 已安装: $nodeVersion"
    exit 0
}

# 优先使用 winget
$hasWinget = Get-Command winget -ErrorAction SilentlyContinue
if ($hasWinget) {
    Write-Host "使用 winget 安装 Node.js..."
    winget install --id OpenJS.NodeJS.LTS --accept-source-agreements --accept-package-agreements
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Node.js 安装成功！"
        exit 0
    }
}

# 备用方案：使用 fnm (Fast Node Manager)，优先国内镜像
Write-Host "尝试使用 fnm 安装 Node.js..."

# 尝试从 npmmirror 安装 fnm
$fnmInstalled = $false
try {
    Write-Host "尝试从国内镜像安装 fnm..."
    $env:FNM_NODE_DIST_MIRROR = "https://npmmirror.com/mirrors/node/"
    irm https://fnm.vercel.app/install.ps1 | iex
    $fnmInstalled = $true
} catch {
    Write-Host "国内镜像失败，尝试官方源..."
    try {
        irm https://fnm.vercel.app/install.ps1 | iex
        $fnmInstalled = $true
    } catch {
        Write-Host "fnm 安装失败: $_"
    }
}

if ($fnmInstalled) {
    # 配置 fnm 环境
    $env:FNM_DIR = "$env:USERPROFILE\.fnm"
    $env:Path = "$env:FNM_DIR;$env:Path"
    $env:FNM_NODE_DIST_MIRROR = "https://npmmirror.com/mirrors/node/"

    # 安装 Node.js 22
    fnm install 22
    fnm default 22
    fnm use 22
}

# 验证安装
$nodeVersion = node --version 2>$null
if ($nodeVersion) {
    Write-Host "Node.js 安装成功: $nodeVersion"
    exit 0
} else {
    Write-Host "Node.js 安装失败"
    exit 1
}
"#;
    
    match shell::run_powershell_output(script) {
        Ok(output) => {
            // 验证安装
            if get_node_version().is_some() {
                Ok(InstallResult {
                    success: true,
                    message: "Node.js 安装成功！请重启应用以使环境变量生效。".to_string(),
                    error: None,
                })
            } else {
                Ok(InstallResult {
                    success: false,
                    message: "安装后需要重启应用".to_string(),
                    error: Some(output),
                })
            }
        }
        Err(e) => Ok(InstallResult {
            success: false,
            message: "Node.js 安装失败".to_string(),
            error: Some(e),
        }),
    }
}

/// macOS 安装 Node.js
async fn install_nodejs_macos() -> Result<InstallResult, String> {
    // 使用 Homebrew 安装，优先 TUNA 镜像
    let script = r#"
# 设置 Homebrew 国内镜像（TUNA）
export HOMEBREW_BREW_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git"
export HOMEBREW_CORE_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git"
export HOMEBREW_BOTTLE_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles"
export HOMEBREW_API_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api"

# 检查 Homebrew
if ! command -v brew &> /dev/null; then
    echo "安装 Homebrew（使用 TUNA 镜像）..."
    /bin/bash -c "$(curl -fsSL https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install/HEAD/install.sh)" || \
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    if [[ -f /opt/homebrew/bin/brew ]]; then
        eval "$(/opt/homebrew/bin/brew shellenv)"
    elif [[ -f /usr/local/bin/brew ]]; then
        eval "$(/usr/local/bin/brew shellenv)"
    fi
fi

echo "安装 Node.js 22..."
brew install node@22
brew link --overwrite node@22

# 验证安装
node --version
"#;
    
    match shell::run_bash_output(script) {
        Ok(output) => Ok(InstallResult {
            success: true,
            message: format!("Node.js 安装成功！{}", output),
            error: None,
        }),
        Err(e) => Ok(InstallResult {
            success: false,
            message: "Node.js 安装失败".to_string(),
            error: Some(e),
        }),
    }
}

/// Linux 安装 Node.js
async fn install_nodejs_linux() -> Result<InstallResult, String> {
    // 优先用 fnm + npmmirror，回退到系统包管理器
    let script = r#"
export FNM_NODE_DIST_MIRROR=https://npmmirror.com/mirrors/node/
export FNM_DIR="$HOME/.fnm"
export PATH="$FNM_DIR:$PATH"

install_via_fnm() {
    # 尝试安装 fnm
    if curl -fsSL https://fnm.vercel.app/install.sh | bash 2>/dev/null; then
        echo "fnm 安装成功"
    else
        return 1
    fi

    # 加载 fnm
    export PATH="$FNM_DIR:$PATH"
    eval "$(fnm env --use-on-cd 2>/dev/null || true)"

    # 安装 Node.js 22（使用 npmmirror）
    FNM_NODE_DIST_MIRROR=https://npmmirror.com/mirrors/node/ fnm install 22
    fnm default 22
    fnm use 22
    node --version
}

# 先尝试 fnm
if install_via_fnm; then
    echo "Node.js 通过 fnm 安装成功"
    exit 0
fi

# 回退到系统包管理器
echo "fnm 安装失败，尝试系统包管理器..."
if command -v apt-get &> /dev/null; then
    curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash - && \
    sudo apt-get install -y nodejs || \
    sudo apt-get install -y nodejs npm
elif command -v dnf &> /dev/null; then
    curl -fsSL https://rpm.nodesource.com/setup_22.x | sudo bash - && \
    sudo dnf install -y nodejs || \
    sudo dnf install -y nodejs npm
elif command -v yum &> /dev/null; then
    curl -fsSL https://rpm.nodesource.com/setup_22.x | sudo bash - && \
    sudo yum install -y nodejs || \
    sudo yum install -y nodejs npm
elif command -v pacman &> /dev/null; then
    sudo pacman -S nodejs npm --noconfirm
else
    echo "无法检测到支持的包管理器"
    exit 1
fi

# 验证安装
node --version
"#;
    
    match shell::run_bash_output(script) {
        Ok(output) => Ok(InstallResult {
            success: true,
            message: format!("Node.js 安装成功！{}", output),
            error: None,
        }),
        Err(e) => Ok(InstallResult {
            success: false,
            message: "Node.js 安装失败".to_string(),
            error: Some(e),
        }),
    }
}

/// 安装 Git
#[command]
pub async fn install_git() -> Result<InstallResult, String> {
    info!("[安装Git] 开始安装 Git...");
    let os = platform::get_os();
    info!("[安装Git] 检测到操作系统: {}", os);

    let result = match os.as_str() {
        "windows" => install_git_windows().await,
        "macos" => install_git_macos().await,
        "linux" => install_git_linux().await,
        _ => Ok(InstallResult {
            success: false,
            message: "不支持的操作系统".to_string(),
            error: Some(format!("不支持的操作系统: {}", os)),
        }),
    };

    match &result {
        Ok(r) if r.success => info!("[安装Git] ✓ 安装成功"),
        Ok(r) => warn!("[安装Git] ✗ 安装失败: {}", r.message),
        Err(e) => error!("[安装Git] ✗ 安装错误: {}", e),
    }

    result
}

async fn install_git_windows() -> Result<InstallResult, String> {
    let script = r#"
$ErrorActionPreference = 'Stop'

$gitVersion = git --version 2>$null
if ($gitVersion) {
    Write-Host "Git 已安装: $gitVersion"
    exit 0
}

$hasWinget = Get-Command winget -ErrorAction SilentlyContinue
if ($hasWinget) {
    Write-Host "使用 winget 安装 Git..."
    winget install --id Git.Git --accept-source-agreements --accept-package-agreements
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Git 安装成功！"
        exit 0
    }
}

Write-Host "Git 安装失败，请手动安装: https://git-scm.com"
exit 1
"#;

    match shell::run_powershell_output(script) {
        Ok(_) => {
            if shell::run_command_output("git", &["--version"]).is_ok() {
                Ok(InstallResult {
                    success: true,
                    message: "Git 安装成功！请重启应用以使环境变量生效。".to_string(),
                    error: None,
                })
            } else {
                Ok(InstallResult {
                    success: false,
                    message: "安装后需要重启应用".to_string(),
                    error: None,
                })
            }
        }
        Err(e) => Ok(InstallResult {
            success: false,
            message: "Git 安装失败".to_string(),
            error: Some(e),
        }),
    }
}

async fn install_git_macos() -> Result<InstallResult, String> {
    let script = r#"
export HOMEBREW_BREW_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git"
export HOMEBREW_CORE_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git"
export HOMEBREW_BOTTLE_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles"
export HOMEBREW_API_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api"

if ! command -v brew &> /dev/null; then
    echo "安装 Homebrew（使用 TUNA 镜像）..."
    /bin/bash -c "$(curl -fsSL https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install/HEAD/install.sh)" || \
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    if [[ -f /opt/homebrew/bin/brew ]]; then
        eval "$(/opt/homebrew/bin/brew shellenv)"
    fi
fi
brew install git
"#;

    match shell::run_bash_output(script) {
        Ok(_) => Ok(InstallResult {
            success: true,
            message: "Git 安装成功！".to_string(),
            error: None,
        }),
        Err(e) => Ok(InstallResult {
            success: false,
            message: "Git 安装失败".to_string(),
            error: Some(e),
        }),
    }
}

async fn install_git_linux() -> Result<InstallResult, String> {
    let script = r#"
if command -v apt-get &> /dev/null; then
    sudo apt-get update && sudo apt-get install -y git
elif command -v dnf &> /dev/null; then
    sudo dnf install -y git
elif command -v yum &> /dev/null; then
    sudo yum install -y git
elif command -v pacman &> /dev/null; then
    sudo pacman -S --noconfirm git
else
    echo "未找到支持的包管理器"
    exit 1
fi
"#;

    match shell::run_bash_output(script) {
        Ok(_) => Ok(InstallResult {
            success: true,
            message: "Git 安装成功！".to_string(),
            error: None,
        }),
        Err(e) => Ok(InstallResult {
            success: false,
            message: "Git 安装失败".to_string(),
            error: Some(e),
        }),
    }
}

// ── 离线 bundle 安装 ──────────────────────────────────────────────────────────

fn resolve_bundled_openclaw_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    let resolver = app.path();
    if let Ok(path) = resolver.resolve("openclaw-bundle", tauri::path::BaseDirectory::Resource) {
        candidates.push(path);
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("src-tauri").join("bundle").join("resources").join("openclaw-bundle"));
        candidates.push(cwd.join("bundle").join("resources").join("openclaw-bundle"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("bundle").join("resources").join("openclaw-bundle"));
            candidates.push(exe_dir.join("resources").join("openclaw-bundle"));
            candidates.push(exe_dir.join("..").join("..").join("Resources").join("openclaw-bundle"));
        }
    }
    for candidate in candidates {
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn bundle_payload_usable(bundle_dir: &PathBuf) -> bool {
    let manifest = bundle_dir.join("manifest.json");
    if !manifest.exists() {
        return false;
    }
    let prefix = bundle_dir.join("prefix");
    if prefix.exists() {
        return true;
    }
    let npm_cli = bundle_dir.join("npm").join("bin").join("npm-cli.js");
    let tgz = bundle_dir.join("openclaw.tgz");
    let cache = bundle_dir.join("npm-cache");
    npm_cli.exists() && tgz.exists() && cache.exists()
}

fn resolve_bundled_node_binary(bundle_dir: &PathBuf) -> Option<PathBuf> {
    let node_bin = if cfg!(target_os = "windows") {
        bundle_dir.join("node").join("node.exe")
    } else {
        bundle_dir.join("node").join("node")
    };
    if node_bin.exists() { Some(node_bin) } else { None }
}

fn prefix_has_openclaw_binary(prefix: &PathBuf) -> bool {
    let candidates: &[&str] = if cfg!(target_os = "windows") {
        // Windows: npm -g --prefix puts bin links in node_modules/.bin/
        &[
            "node_modules/.bin/openclaw.cmd",
            "node_modules/.bin/openclaw",
            "openclaw.cmd",
            "bin/openclaw.cmd",
        ]
    } else {
        &["bin/openclaw"]
    };
    candidates.iter().any(|rel| prefix.join(rel).exists())
}

fn copy_dir_recursive_counted(src: &Path, dst: &Path, count: &mut usize) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive_counted(&src_path, &dst_path, count)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                if let Ok(meta) = fs::metadata(&src_path) {
                    let _ = fs::set_permissions(&dst_path, meta.permissions());
                }
            }
            *count += 1;
        }
    }
    Ok(())
}

/// 把 bundle 里的 node 二进制复制到 prefix/node/ 目录，供 openclaw 运行时使用
fn copy_bundled_node_to_prefix(bundle_dir: &PathBuf, prefix: &PathBuf) -> Result<(), String> {
    let Some(node_bin) = resolve_bundled_node_binary(bundle_dir) else {
        return Ok(()); // 没有 bundled node，跳过
    };
    let node_dir = prefix.join("node");
    fs::create_dir_all(&node_dir).map_err(|e| e.to_string())?;
    let node_name = if cfg!(target_os = "windows") { "node.exe" } else { "node" };
    let dest = node_dir.join(node_name);
    fs::copy(&node_bin, &dest).map_err(|e| format!("复制 node 失败: {}", e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("设置 node 权限失败: {}", e))?;
    }
    info!("[离线安装] bundled node 已复制到 {}", dest.display());
    Ok(())
}

fn emit_progress(app: &tauri::AppHandle, step: &str, progress: u8, message: &str) {
    let _ = app.emit("install-progress", InstallProgress {
        step: step.to_string(),
        progress,
        message: message.to_string(),
        error: None,
    });
}

fn install_openclaw_from_bundle_dir(app: &tauri::AppHandle, bundle_dir: &PathBuf, install_dir: Option<&Path>) -> Result<bool, String> {
    if !bundle_payload_usable(bundle_dir) {
        info!("[离线安装] bundle payload 不完整，跳过离线安装");
        return Ok(false);
    }

    let prefix = if let Some(dir) = install_dir {
        dir.to_path_buf()
    } else {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| "无法获取用户主目录".to_string())?;
        PathBuf::from(home).join(".openclaw")
    };
    emit_progress(app, "prepare", 5, "正在准备安装目录...");
    fs::create_dir_all(&prefix).map_err(|e| e.to_string())?;

    let prepared_prefix = bundle_dir.join("prefix");
    if prepared_prefix.exists() {
        info!("[离线安装] 从 bundled prefix snapshot 安装...");
        emit_progress(app, "copy", 20, "正在复制文件（这可能需要一两分钟）...");
        let mut count = 0usize;
        copy_dir_recursive_counted(&prepared_prefix, &prefix, &mut count)?;
        emit_progress(app, "node", 80, &format!("已复制 {} 个文件，正在复制 Node.js 运行时...", count));
        copy_bundled_node_to_prefix(bundle_dir, &prefix)?;
        if prefix_has_openclaw_binary(&prefix) {
            info!("[离线安装] ✓ bundled prefix 安装完成");
            emit_progress(app, "done", 100, "安装完成！");
            return Ok(true);
        }
        warn!("[离线安装] prefix 复制完成但未找到 openclaw binary，尝试 npm 离线安装");
    }

    let Some(node_bin) = resolve_bundled_node_binary(bundle_dir) else {
        info!("[离线安装] 未找到 bundled node，跳过离线安装");
        return Ok(false);
    };
    let npm_cli = bundle_dir.join("npm").join("bin").join("npm-cli.js");
    let openclaw_tgz = bundle_dir.join("openclaw.tgz");
    let npm_cache = bundle_dir.join("npm-cache");

    if !npm_cli.exists() || !openclaw_tgz.exists() || !npm_cache.exists() {
        info!("[离线安装] bundle payload 不完整，跳过离线安装");
        return Ok(false);
    }

    info!("[离线安装] 使用 bundled npm 离线安装 openclaw...");
    emit_progress(app, "npm", 40, "正在执行 npm 安装（可能需要几分钟）...");
    let output = Command::new(&node_bin)
        .arg(&npm_cli)
        .arg("install")
        .arg("-g")
        .arg("--prefix").arg(&prefix)
        .arg(&openclaw_tgz)
        .arg("--cache").arg(&npm_cache)
        .arg("--offline")
        .arg("--no-audit")
        .arg("--no-fund")
        .arg("--loglevel=error")
        .output()
        .map_err(|e| format!("运行 bundled npm 失败: {}", e))?;

    if output.status.success() {
        emit_progress(app, "node", 85, "正在复制 Node.js 运行时...");
        copy_bundled_node_to_prefix(bundle_dir, &prefix)?;
        if prefix_has_openclaw_binary(&prefix) {
            info!("[离线安装] ✓ npm 离线安装完成");
            emit_progress(app, "done", 100, "安装完成！");
            return Ok(true);
        }
        return Err("npm 离线安装成功但未找到 openclaw binary".to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(format!("npm 离线安装失败: {} {}", stdout.trim(), stderr.trim()))
}

fn try_install_openclaw_offline(app: &tauri::AppHandle, install_dir: Option<&Path>) -> Option<bool> {
    let bundle_dir = resolve_bundled_openclaw_dir(app)?;
    info!("[离线安装] 找到 bundle: {}", bundle_dir.display());
    match install_openclaw_from_bundle_dir(app, &bundle_dir, install_dir) {
        Ok(result) => Some(result),
        Err(e) => {
            warn!("[离线安装] 失败: {}", e);
            None
        }
    }
}

// ── bundle 下载安装 ────────────────────────────────────────────────────────────

/// 内置 GitHub 代理镜像列表
const GHPROXY_LIST: &[&str] = &[
    "https://ghproxy.monkeyray.net/",
    "https://gh.xxooo.cf/",
    "https://fastgit.cc/",
    "https://ghproxy.cxkpro.top/",
    "https://gh.idayer.com/",
];

/// 返回不带代理的 GitHub 原始 bundle URL
fn get_base_github_url() -> String {
    let os = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        _ => "linux",
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        _ => "x64",
    };
    let ext = if cfg!(target_os = "windows") { "zip" } else { "tar.gz" };
    format!(
        "https://github.com/icepie/openclaw-manager/releases/download/dev/openclaw-bundle-{}-{}.{}",
        os, arch, ext
    )
}

/// 返回当前平台对应的 bundle 默认下载 URL（使用第一个代理）
#[command]
pub fn get_bundle_download_url() -> String {
    format!("{}{}", GHPROXY_LIST[0], get_base_github_url())
}

/// 并发测试所有代理节点，返回响应最快的完整 URL
#[command]
pub async fn select_fastest_proxy() -> String {
    let base_url = get_base_github_url();
    info!("[代理测速] 开始测试 {} 个节点...", GHPROXY_LIST.len());

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return format!("{}{}", GHPROXY_LIST[0], base_url),
    };

    let mut handles = Vec::new();
    for &proxy in GHPROXY_LIST {
        let url = format!("{}{}", proxy, base_url);
        let c = client.clone();
        let proxy_owned = proxy.to_string();
        handles.push(tokio::spawn(async move {
            let start = std::time::Instant::now();
            let result = c.head(&url).send().await;
            let latency = start.elapsed().as_millis();
            match result {
                Ok(resp) if resp.status().as_u16() < 400 => {
                    info!("[代理测速] {} → {}ms ({})", proxy_owned, latency, resp.status());
                    Some((proxy_owned, latency))
                }
                Ok(resp) => {
                    info!("[代理测速] {} → 失败 ({})", proxy_owned, resp.status());
                    None
                }
                Err(e) => {
                    info!("[代理测速] {} → 错误: {}", proxy_owned, e);
                    None
                }
            }
        }));
    }

    let mut fastest: Option<(String, u128)> = None;
    for handle in handles {
        if let Ok(Some((proxy, latency))) = handle.await {
            match &fastest {
                None => fastest = Some((proxy, latency)),
                Some((_, best)) if latency < *best => fastest = Some((proxy, latency)),
                _ => {}
            }
        }
    }

    match fastest {
        Some((proxy, latency)) => {
            info!("[代理测速] 最快节点: {} ({}ms)", proxy, latency);
            format!("{}{}", proxy, base_url)
        }
        None => {
            info!("[代理测速] 所有节点不可用，使用默认");
            format!("{}{}", GHPROXY_LIST[0], base_url)
        }
    }
}

/// 下载进度事件 payload
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: Option<f64>,
}

/// 用 reqwest 下载文件，支持断点续传，通过 Tauri 事件推送进度
async fn download_file(
    app: &tauri::AppHandle,
    url: &str,
    out: &PathBuf,
) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;
    use futures_util::StreamExt;

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = out.with_extension("download");

    // 检查已下载的字节数（断点续传）
    let resume_from = if tmp.exists() {
        tmp.metadata().map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    let client = reqwest::Client::builder()
        .user_agent("openclaw-manager")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let mut req = client.get(url);
    if resume_from > 0 {
        info!("[下载] 断点续传，从 {} 字节继续", resume_from);
        req = req.header("Range", format!("bytes={}-", resume_from));
    }

    let resp = req.send().await
        .map_err(|e| format!("HTTP 请求失败: {}", e))?;

    let status = resp.status();
    // 206 = Partial Content（断点续传成功），200 = 全量下载
    if !status.is_success() {
        return Err(format!("下载失败，HTTP {}", status));
    }

    let total = resp.content_length().map(|len| {
        if resume_from > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT {
            len + resume_from
        } else {
            len
        }
    });

    // 追加模式（断点续传）或覆盖模式
    let append = resume_from > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT;
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(append)
        .write(!append)
        .truncate(!append)
        .open(&tmp)
        .await
        .map_err(|e| format!("打开临时文件失败: {}", e))?;

    let mut downloaded = if append { resume_from } else { 0u64 };
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if INSTALL_CANCELLED.load(Ordering::Relaxed) {
            return Err("已取消".to_string());
        }
        let chunk = chunk.map_err(|e| format!("读取数据失败: {}", e))?;
        file.write_all(&chunk).await
            .map_err(|e| format!("写入文件失败: {}", e))?;
        downloaded += chunk.len() as u64;

        let percent = total.map(|t| if t > 0 { downloaded as f64 / t as f64 * 100.0 } else { 0.0 });
        let _ = app.emit("bundle-download-progress", DownloadProgress {
            downloaded,
            total,
            percent,
        });
    }

    file.flush().await.map_err(|e| format!("刷新文件失败: {}", e))?;
    drop(file);

    let _ = fs::remove_file(out);
    fs::rename(&tmp, out).map_err(|e| format!("重命名文件失败: {}", e))?;
    Ok(())
}

fn extract_tar_gz(archive: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    use flate2::read::GzDecoder;
    use tar::Archive;
    if INSTALL_CANCELLED.load(Ordering::Relaxed) {
        return Err("已取消".to_string());
    }
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let file = fs::File::open(archive).map_err(|e| format!("打开压缩包失败: {}", e))?;
    let gz = GzDecoder::new(file);
    let mut ar = Archive::new(gz);
    ar.set_preserve_permissions(true);
    ar.unpack(dest).map_err(|e| format!("tar.gz 解压失败: {}", e))
}

fn extract_zip(archive: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    use zip::ZipArchive;
    use rayon::prelude::*;
    if INSTALL_CANCELLED.load(Ordering::Relaxed) {
        return Err("已取消".to_string());
    }
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;

    let file = fs::File::open(archive).map_err(|e| format!("打开压缩包失败: {}", e))?;
    let mut zip = ZipArchive::new(file).map_err(|e| format!("读取 zip 失败: {}", e))?;

    // First pass: create all directories
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|e| format!("读取 zip 条目失败: {}", e))?;
        if entry.is_dir() {
            fs::create_dir_all(dest.join(entry.mangled_name())).map_err(|e| e.to_string())?;
        }
    }

    // Second pass: collect file entries (index + path + data) for parallel write
    let entries: Vec<(PathBuf, Vec<u8>, Option<u32>)> = (0..zip.len())
        .filter_map(|i| {
            let mut entry = zip.by_index(i).ok()?;
            if entry.is_dir() { return None; }
            let out_path = dest.join(entry.mangled_name());
            let mut data = Vec::with_capacity(entry.size() as usize);
            std::io::copy(&mut entry, &mut data).ok()?;
            #[cfg(unix)]
            let mode = entry.unix_mode();
            #[cfg(not(unix))]
            let mode: Option<u32> = None;
            Some((out_path, data, mode))
        })
        .collect();

    // Parallel write
    let cancelled = &INSTALL_CANCELLED;
    entries.par_iter().try_for_each(|(out_path, data, mode)| -> Result<(), String> {
        if cancelled.load(Ordering::Relaxed) {
            return Err("已取消".to_string());
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(out_path, data).map_err(|e| format!("写入文件失败: {}", e))?;
        #[cfg(unix)]
        if let Some(m) = mode {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(out_path, fs::Permissions::from_mode(*m));
        }
        Ok(())
    })
}

async fn download_and_install_bundle(app: &tauri::AppHandle, url: &str, install_dir: Option<&Path>) -> Result<(), String> {
    let cache_dir = {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| "无法获取用户主目录".to_string())?;
        PathBuf::from(home).join(".openclaw").join("bundle-cache")
    };
    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;

    let is_zip = url.ends_with(".zip");
    let archive_name = if is_zip { "openclaw-bundle.zip" } else { "openclaw-bundle.tar.gz" };
    let archive = cache_dir.join(archive_name);
    let extract_dir = cache_dir.join("extract");

    info!("[下载安装] 下载 bundle: {}", url);
    download_file(app, url, &archive).await?;

    info!("[下载安装] 解压 bundle...");
    emit_progress(app, "extract", 55, "正在解压 bundle（文件较多，请耐心等待）...");
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).map_err(|e| e.to_string())?;
    }
    if is_zip {
        extract_zip(&archive, &extract_dir)?;
    } else {
        extract_tar_gz(&archive, &extract_dir)?;
    }

    // 找到解压后的 openclaw-bundle 目录
    let bundle_dir = if extract_dir.join("openclaw-bundle").exists() {
        extract_dir.join("openclaw-bundle")
    } else {
        extract_dir.clone()
    };

    info!("[下载安装] 从下载的 bundle 安装...");
    match install_openclaw_from_bundle_dir(app, &bundle_dir, install_dir) {
        Ok(true) => {
            let _ = fs::remove_file(&archive);
            let _ = fs::remove_dir_all(&extract_dir);
            Ok(())
        }
        Ok(false) => Err("bundle 安装失败：payload 不完整".to_string()),
        Err(e) => Err(e),
    }
}

// ── 安装 OpenClaw ─────────────────────────────────────────────────────────────

#[command]
pub async fn install_openclaw(app: tauri::AppHandle, bundle_url: Option<String>, local_bundle_path: Option<String>, install_dir: Option<String>) -> Result<InstallResult, String> {
    info!("[安装OpenClaw] 开始安装 OpenClaw...");
    INSTALL_CANCELLED.store(false, Ordering::Relaxed);

    let dir: Option<PathBuf> = install_dir.map(PathBuf::from);
    let dir_ref: Option<&Path> = dir.as_deref();

    // 1. 用户指定本地离线包
    if let Some(ref local_path) = local_bundle_path {
        info!("[安装OpenClaw] 使用本地离线包: {}", local_path);
        let archive = PathBuf::from(local_path);
        let cache_dir = {
            let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))
                .map_err(|_| "无法获取用户主目录".to_string())?;
            PathBuf::from(home).join(".openclaw").join("bundle-cache")
        };
        fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
        let extract_dir = cache_dir.join("extract");
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir).map_err(|e| e.to_string())?;
        }
        emit_progress(&app, "extract", 10, "正在解压离线包...");
        if local_path.ends_with(".zip") {
            extract_zip(&archive, &extract_dir)?;
        } else {
            extract_tar_gz(&archive, &extract_dir)?;
        }
        let bundle_dir = if extract_dir.join("openclaw-bundle").exists() {
            extract_dir.join("openclaw-bundle")
        } else {
            extract_dir.clone()
        };
        return match install_openclaw_from_bundle_dir(&app, &bundle_dir, dir_ref) {
            Ok(true) => {
                let _ = fs::remove_dir_all(&extract_dir);
                Ok(InstallResult { success: true, message: "OpenClaw 本地离线包安装成功！".to_string(), error: None })
            }
            Ok(false) => Ok(InstallResult { success: false, message: "本地离线包安装失败：payload 不完整".to_string(), error: Some("payload 不完整".to_string()) }),
            Err(e) => Ok(InstallResult { success: false, message: format!("本地离线包安装失败: {}", e), error: Some(e) }),
        };
    }

    // 2. 优先 app 内嵌 bundle
    if let Some(true) = try_install_openclaw_offline(&app, dir_ref) {
        info!("[安装OpenClaw] ✓ 本地 bundle 安装成功");
        return Ok(InstallResult {
            success: true,
            message: "OpenClaw 安装成功！".to_string(),
            error: None,
        });
    }

    // 3. 从指定 URL（或默认）下载 bundle 安装
    let url = bundle_url.unwrap_or_else(get_bundle_download_url);
    info!("[安装OpenClaw] 从远程下载: {}", url);
    match download_and_install_bundle(&app, &url, dir_ref).await {
        Ok(()) => {
            info!("[安装OpenClaw] ✓ 下载安装成功");
            Ok(InstallResult {
                success: true,
                message: "OpenClaw 下载安装成功！".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("[安装OpenClaw] ✗ 下载安装失败: {}", e);
            Ok(InstallResult {
                success: false,
                message: format!("安装失败: {}", e),
                error: Some(e),
            })
        }
    }
}

/// 初始化 OpenClaw 配置
#[command]
pub async fn init_openclaw_config() -> Result<InstallResult, String> {
    info!("[初始化配置] 开始初始化 OpenClaw 配置...");
    
    let config_dir = platform::get_config_dir();
    info!("[初始化配置] 配置目录: {}", config_dir);
    
    // 创建配置目录
    info!("[初始化配置] 创建配置目录...");
    if let Err(e) = std::fs::create_dir_all(&config_dir) {
        error!("[初始化配置] ✗ 创建配置目录失败: {}", e);
        return Ok(InstallResult {
            success: false,
            message: "创建配置目录失败".to_string(),
            error: Some(e.to_string()),
        });
    }
    
    // 创建子目录
    let subdirs = ["agents/main/sessions", "agents/main/agent", "credentials"];
    for subdir in subdirs {
        let path = format!("{}/{}", config_dir, subdir);
        info!("[初始化配置] 创建子目录: {}", subdir);
        if let Err(e) = std::fs::create_dir_all(&path) {
            error!("[初始化配置] ✗ 创建目录失败: {} - {}", subdir, e);
            return Ok(InstallResult {
                success: false,
                message: format!("创建目录失败: {}", subdir),
                error: Some(e.to_string()),
            });
        }
    }
    
    // 设置配置目录权限为 700（与 shell 脚本 chmod 700 一致）
    // 仅在 Unix 系统上执行
    #[cfg(unix)]
    {
        info!("[初始化配置] 设置目录权限为 700...");
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&config_dir) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o700);
            if let Err(e) = std::fs::set_permissions(&config_dir, perms) {
                warn!("[初始化配置] 设置权限失败: {}", e);
            } else {
                info!("[初始化配置] ✓ 权限设置成功");
            }
        }
    }
    
    // 直接写入 openclaw.json 配置文件（不依赖 openclaw config set 命令）
    let config_file = format!("{}/openclaw.json", config_dir);
    let config_exists = std::path::Path::new(&config_file).exists();
    if !config_exists {
        info!("[初始化配置] 写入默认 openclaw.json...");
        let default_config = serde_json::json!({
            "gateway": {
                "mode": "local"
            },
            "plugins": {
                "allow": ["@openclaw-china/dingtalk"]
            }
        });
        if let Err(e) = std::fs::write(&config_file, serde_json::to_string_pretty(&default_config).unwrap()) {
            warn!("[初始化配置] 写入 openclaw.json 失败: {}", e);
        } else {
            info!("[初始化配置] ✓ openclaw.json 写入成功");
        }
    } else {
        // 文件已存在，尝试确保 gateway.mode=local 和 plugins.allow
        info!("[初始化配置] openclaw.json 已存在，尝试设置 gateway.mode 和 plugins.allow...");
        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                if json.get("gateway").and_then(|g| g.get("mode")).is_none() {
                    json["gateway"]["mode"] = serde_json::json!("local");
                }
                // 确保 plugins.allow 包含 dingtalk
                let allow = json["plugins"]["allow"].as_array_mut();
                let dingtalk = "@openclaw-china/dingtalk";
                match allow {
                    Some(arr) => {
                        if !arr.iter().any(|v| v.as_str() == Some(dingtalk)) {
                            arr.push(serde_json::json!(dingtalk));
                        }
                    }
                    None => {
                        json["plugins"]["allow"] = serde_json::json!([dingtalk]);
                    }
                }
                if let Ok(updated) = serde_json::to_string_pretty(&json) {
                    let _ = std::fs::write(&config_file, updated);
                    info!("[初始化配置] ✓ gateway.mode 和 plugins.allow 已设置");
                }
            }
        }
    }

    // 设置 gateway mode 为 local（通过命令，作为补充）
    info!("[初始化配置] 执行: openclaw config set gateway.mode local");
    let result = shell::run_openclaw(&["config", "set", "gateway.mode", "local"]);
    
    match result {
        Ok(output) => {
            info!("[初始化配置] ✓ 配置初始化成功");
            debug!("[初始化配置] 命令输出: {}", output);
        },
        Err(e) => {
            // 命令失败不影响结果，配置文件已直接写入
            warn!("[初始化配置] openclaw config set 失败（已直接写入配置文件）: {}", e);
        },
    }

    Ok(InstallResult {
        success: true,
        message: "配置初始化成功！".to_string(),
        error: None,
    })
}

/// 打开终端执行安装脚本（用于需要管理员权限的场景）
#[command]
pub async fn open_install_terminal(install_type: String) -> Result<String, String> {
    match install_type.as_str() {
        "nodejs" => open_nodejs_install_terminal().await,
        "openclaw" => open_openclaw_install_terminal().await,
        _ => Err(format!("未知的安装类型: {}", install_type)),
    }
}

/// 打开终端安装 Node.js
async fn open_nodejs_install_terminal() -> Result<String, String> {
    if platform::is_windows() {
        // Windows: 打开 PowerShell 执行安装
        let script = r#"
Start-Process powershell -ArgumentList '-NoExit', '-Command', '
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "    Node.js 安装向导" -ForegroundColor White
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 检查 winget
$hasWinget = Get-Command winget -ErrorAction SilentlyContinue
if ($hasWinget) {
    Write-Host "正在使用 winget 安装 Node.js 22..." -ForegroundColor Yellow
    winget install --id OpenJS.NodeJS.LTS --accept-source-agreements --accept-package-agreements
} else {
    Write-Host "请从以下地址下载安装 Node.js:" -ForegroundColor Yellow
    Write-Host "https://nodejs.org/en/download" -ForegroundColor Green
    Write-Host ""
    Start-Process "https://nodejs.org/en/download"
}

Write-Host ""
Write-Host "安装完成后请重启 OpenClaw Manager" -ForegroundColor Green
Write-Host ""
Read-Host "按回车键关闭此窗口"
' -Verb RunAs
"#;
        shell::run_powershell_output(script)?;
        Ok("已打开安装终端".to_string())
    } else if platform::is_macos() {
        // macOS: 打开 Terminal.app
        let script_content = r#"#!/bin/bash
clear
echo "========================================"
echo "    Node.js 安装向导"
echo "========================================"
echo ""

# 设置 Homebrew 国内镜像（TUNA）
export HOMEBREW_BREW_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git"
export HOMEBREW_CORE_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git"
export HOMEBREW_BOTTLE_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles"
export HOMEBREW_API_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api"

# 检查 Homebrew
if ! command -v brew &> /dev/null; then
    echo "正在安装 Homebrew（使用 TUNA 镜像）..."
    /bin/bash -c "$(curl -fsSL https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install/HEAD/install.sh)" || \
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    if [[ -f /opt/homebrew/bin/brew ]]; then
        eval "$(/opt/homebrew/bin/brew shellenv)"
    elif [[ -f /usr/local/bin/brew ]]; then
        eval "$(/usr/local/bin/brew shellenv)"
    fi
fi

echo "正在安装 Node.js 22..."
brew install node@22
brew link --overwrite node@22

echo ""
echo "安装完成！"
node --version
echo ""
read -p "按回车键关闭此窗口..."
"#;
        
        let script_path = "/tmp/openclaw_install_nodejs.command";
        std::fs::write(script_path, script_content)
            .map_err(|e| format!("创建脚本失败: {}", e))?;
        
        std::process::Command::new("chmod")
            .args(["+x", script_path])
            .output()
            .map_err(|e| format!("设置权限失败: {}", e))?;
        
        std::process::Command::new("open")
            .arg(script_path)
            .spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;
        
        Ok("已打开安装终端".to_string())
    } else {
        Err("请手动安装 Node.js: https://nodejs.org/".to_string())
    }
}

/// 打开终端安装 OpenClaw
async fn open_openclaw_install_terminal() -> Result<String, String> {
    if platform::is_windows() {
        let script = r#"
Start-Process powershell -ArgumentList '-NoExit', '-Command', '
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "    OpenClaw 安装向导" -ForegroundColor White
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "正在安装 OpenClaw（国内镜像）..." -ForegroundColor Yellow
npm install -g openclaw@latest --registry https://registry.npmmirror.com
if ($LASTEXITCODE -ne 0) {
    Write-Host "国内镜像失败，尝试官方源..." -ForegroundColor Yellow
    npm install -g openclaw@latest
}

Write-Host ""
Write-Host "初始化配置..."
openclaw config set gateway.mode local

Write-Host ""
Write-Host "安装完成！" -ForegroundColor Green
openclaw --version
Write-Host ""
Read-Host "按回车键关闭此窗口"
'
"#;
        shell::run_powershell_output(script)?;
        Ok("已打开安装终端".to_string())
    } else if platform::is_macos() {
        let script_content = r#"#!/bin/bash
clear
echo "========================================"
echo "    OpenClaw 安装向导"
echo "========================================"
echo ""

echo "正在安装 OpenClaw（国内镜像）..."
npm install -g openclaw@latest --registry https://registry.npmmirror.com || \
npm install -g openclaw@latest

echo ""
echo "初始化配置..."
openclaw config set gateway.mode local 2>/dev/null || true

mkdir -p ~/.openclaw/agents/main/sessions
mkdir -p ~/.openclaw/agents/main/agent
mkdir -p ~/.openclaw/credentials

echo ""
echo "安装完成！"
openclaw --version
echo ""
read -p "按回车键关闭此窗口..."
"#;
        
        let script_path = "/tmp/openclaw_install_openclaw.command";
        std::fs::write(script_path, script_content)
            .map_err(|e| format!("创建脚本失败: {}", e))?;
        
        std::process::Command::new("chmod")
            .args(["+x", script_path])
            .output()
            .map_err(|e| format!("设置权限失败: {}", e))?;
        
        std::process::Command::new("open")
            .arg(script_path)
            .spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;
        
        Ok("已打开安装终端".to_string())
    } else {
        // Linux
        let script_content = r#"#!/bin/bash
clear
echo "========================================"
echo "    OpenClaw 安装向导"
echo "========================================"
echo ""

echo "正在安装 OpenClaw（国内镜像）..."
npm install -g openclaw@latest --registry https://registry.npmmirror.com || \
npm install -g openclaw@latest

echo ""
echo "初始化配置..."
openclaw config set gateway.mode local 2>/dev/null || true

mkdir -p ~/.openclaw/agents/main/sessions
mkdir -p ~/.openclaw/agents/main/agent
mkdir -p ~/.openclaw/credentials

echo ""
echo "安装完成！"
openclaw --version
echo ""
read -p "按回车键关闭..."
"#;
        
        let script_path = "/tmp/openclaw_install_openclaw.sh";
        std::fs::write(script_path, script_content)
            .map_err(|e| format!("创建脚本失败: {}", e))?;
        
        std::process::Command::new("chmod")
            .args(["+x", script_path])
            .output()
            .map_err(|e| format!("设置权限失败: {}", e))?;
        
        // 尝试不同的终端
        let terminals = ["gnome-terminal", "xfce4-terminal", "konsole", "xterm"];
        for term in terminals {
            if std::process::Command::new(term)
                .args(["--", script_path])
                .spawn()
                .is_ok()
            {
                return Ok("已打开安装终端".to_string());
            }
        }
        
        Err("无法启动终端，请手动运行: npm install -g openclaw".to_string())
    }
}

/// 取消正在进行的安装
#[command]
pub async fn cancel_install() {
    info!("[安装] 用户取消安装");
    INSTALL_CANCELLED.store(true, Ordering::Relaxed);
}

/// 卸载 OpenClaw
#[command]
pub async fn uninstall_openclaw() -> Result<InstallResult, String> {
    info!("[卸载OpenClaw] 开始卸载 OpenClaw...");

    // 先停止服务
    info!("[卸载OpenClaw] 尝试停止服务...");
    let _ = shell::run_openclaw(&["gateway", "stop"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "无法获取用户主目录".to_string())?;
    let prefix = PathBuf::from(&home).join(".openclaw");

    // 删除离线安装的文件（保留配置文件和日志）
    let remove_dirs = ["node", "node_modules", "bin", "lib", "bundle-cache"];
    let mut removed = vec![];
    for dir in &remove_dirs {
        let p = prefix.join(dir);
        if p.exists() {
            if let Err(e) = std::fs::remove_dir_all(&p) {
                warn!("[卸载OpenClaw] 删除 {} 失败: {}", dir, e);
            } else {
                info!("[卸载OpenClaw] ✓ 已删除 {}", p.display());
                removed.push(*dir);
            }
        }
    }
    // 删除根目录下的 .cmd / .exe 可执行文件
    if let Ok(entries) = std::fs::read_dir(&prefix) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".cmd") || name.ends_with(".exe") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    // 尝试 npm uninstall 作为补充（全局安装场景）
    if platform::is_windows() {
        let _ = shell::run_cmd_output("npm uninstall -g openclaw");
    } else {
        let _ = shell::run_bash_output("npm uninstall -g openclaw 2>/dev/null");
    }

    if get_openclaw_version().is_none() {
        info!("[卸载OpenClaw] ✓ 卸载成功");
        Ok(InstallResult {
            success: true,
            message: format!("OpenClaw 已成功卸载！（已删除: {}）", removed.join(", ")),
            error: None,
        })
    } else {
        warn!("[卸载OpenClaw] 卸载后 openclaw 仍可检测到");
        Ok(InstallResult {
            success: false,
            message: "卸载命令已执行，但 OpenClaw 仍然存在，请尝试手动删除 ~/.openclaw 目录".to_string(),
            error: None,
        })
    }
}

/// 版本更新信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// 是否有更新可用
    pub update_available: bool,
    /// 当前版本
    pub current_version: Option<String>,
    /// 最新版本
    pub latest_version: Option<String>,
    /// 错误信息
    pub error: Option<String>,
}

/// 检查 OpenClaw 更新
#[command]
pub async fn check_openclaw_update() -> Result<UpdateInfo, String> {
    info!("[版本检查] 开始检查 OpenClaw 更新...");
    
    // 获取当前版本
    let current_version = get_openclaw_version();
    info!("[版本检查] 当前版本: {:?}", current_version);
    
    if current_version.is_none() {
        info!("[版本检查] OpenClaw 未安装");
        return Ok(UpdateInfo {
            update_available: false,
            current_version: None,
            latest_version: None,
            error: Some("OpenClaw 未安装".to_string()),
        });
    }
    
    // 获取最新版本
    let latest_version = get_latest_openclaw_version();
    info!("[版本检查] 最新版本: {:?}", latest_version);
    
    if latest_version.is_none() {
        return Ok(UpdateInfo {
            update_available: false,
            current_version,
            latest_version: None,
            error: Some("无法获取最新版本信息".to_string()),
        });
    }
    
    // 比较版本
    let current = current_version.clone().unwrap();
    let latest = latest_version.clone().unwrap();
    let update_available = compare_versions(&current, &latest);
    
    info!("[版本检查] 是否有更新: {}", update_available);
    
    Ok(UpdateInfo {
        update_available,
        current_version,
        latest_version,
        error: None,
    })
}

/// 获取 npm registry 上的最新版本
fn get_latest_openclaw_version() -> Option<String> {
    // 使用 npm view 获取最新版本
    let result = if platform::is_windows() {
        shell::run_cmd_output("npm view openclaw version")
    } else {
        shell::run_bash_output("npm view openclaw version 2>/dev/null")
    };
    
    match result {
        Ok(version) => {
            let v = version.trim().to_string();
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        }
        Err(e) => {
            warn!("[版本检查] 获取最新版本失败: {}", e);
            None
        }
    }
}

/// 比较版本号，返回是否有更新可用
/// current: 当前版本 (如 "1.0.0" 或 "v1.0.0")
/// latest: 最新版本 (如 "1.0.1")
fn compare_versions(current: &str, latest: &str) -> bool {
    // 移除可能的 'v' 前缀和空白
    let current = current.trim().trim_start_matches('v');
    let latest = latest.trim().trim_start_matches('v');
    
    // 分割版本号
    let current_parts: Vec<u32> = current
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let latest_parts: Vec<u32> = latest
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    // 比较每个部分
    for i in 0..3 {
        let c = current_parts.get(i).unwrap_or(&0);
        let l = latest_parts.get(i).unwrap_or(&0);
        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }
    
    false
}

/// 更新 OpenClaw
#[command]
pub async fn update_openclaw() -> Result<InstallResult, String> {
    info!("[更新OpenClaw] 开始更新 OpenClaw...");
    let os = platform::get_os();
    
    // 先停止服务
    info!("[更新OpenClaw] 尝试停止服务...");
    let _ = shell::run_openclaw(&["gateway", "stop"]);
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    let result = match os.as_str() {
        "windows" => {
            info!("[更新OpenClaw] 使用 Windows 更新方式...");
            update_openclaw_windows().await
        },
        _ => {
            info!("[更新OpenClaw] 使用 Unix 更新方式 (npm)...");
            update_openclaw_unix().await
        },
    };
    
    match &result {
        Ok(r) if r.success => info!("[更新OpenClaw] ✓ 更新成功"),
        Ok(r) => warn!("[更新OpenClaw] ✗ 更新失败: {}", r.message),
        Err(e) => error!("[更新OpenClaw] ✗ 更新错误: {}", e),
    }
    
    result
}

/// Windows 更新 OpenClaw
async fn update_openclaw_windows() -> Result<InstallResult, String> {
    info!("[更新OpenClaw] 执行 npm install -g openclaw@latest...");
    
    match shell::run_cmd_output("npm install -g openclaw@latest") {
        Ok(output) => {
            info!("[更新OpenClaw] npm 输出: {}", output);
            
            // 获取新版本
            let new_version = get_openclaw_version();
            
            Ok(InstallResult {
                success: true,
                message: format!("OpenClaw 已更新到 {}", new_version.unwrap_or("最新版本".to_string())),
                error: None,
            })
        }
        Err(e) => {
            warn!("[更新OpenClaw] npm install 失败: {}", e);
            Ok(InstallResult {
                success: false,
                message: "OpenClaw 更新失败".to_string(),
                error: Some(e),
            })
        }
    }
}

/// Unix 系统更新 OpenClaw
async fn update_openclaw_unix() -> Result<InstallResult, String> {
    let script = r#"
echo "更新 OpenClaw..."
npm install -g openclaw@latest

# 验证更新
openclaw --version
"#;
    
    match shell::run_bash_output(script) {
        Ok(output) => Ok(InstallResult {
            success: true,
            message: format!("OpenClaw 已更新！{}", output),
            error: None,
        }),
        Err(e) => Ok(InstallResult {
            success: false,
            message: "OpenClaw 更新失败".to_string(),
            error: Some(e),
        }),
    }
}

/// 打开一个已注入 ~/.openclaw/bin 和 ~/.openclaw/node 到 PATH 的终端
#[command]
pub async fn open_env_terminal() -> Result<String, String> {
    let config_dir = platform::get_config_dir();
    let env_file = platform::get_env_file_path();

    if platform::is_windows() {
        #[cfg(windows)]
        {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // Build env var injections for PowerShell
        // Use single quotes for PATH to avoid quoting issues with backslashes/spaces
        let win_path = extended_path.replace('/', "\\");
        let mut ps_lines = vec![
            format!("$env:PATH = '{}' + ';' + $env:PATH", win_path.replace('\'', "''")),
        ];
        // Inject user env vars from env file
        let env_content = std::fs::read_to_string(&env_file).unwrap_or_default();
        for line in env_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            let line = line.strip_prefix("export ").unwrap_or(line);
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                // Use single quotes, escape any single quotes in value
                ps_lines.push(format!("$env:{} = '{}'", k.trim(), v.replace('\'', "''")));
            }
        }
        ps_lines.push("Write-Host 'OpenClaw 环境已就绪' -ForegroundColor Green".to_string());
        ps_lines.push(format!("Write-Host '配置目录: {}' -ForegroundColor Cyan", config_dir.replace('\'', "''")));

        let ps_cmd = ps_lines.join("; ");
        // Use -EncodedCommand to avoid all quoting issues
        use std::io::Write;
        let encoded: Vec<u16> = ps_cmd.encode_utf16().collect();
        let mut bytes = Vec::with_capacity(encoded.len() * 2);
        for c in &encoded {
            bytes.write_all(&c.to_le_bytes()).ok();
        }
        let b64 = {
            use std::fmt::Write as FmtWrite;
            let mut s = String::new();
            const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut i = 0;
            while i + 2 < bytes.len() {
                let b0 = bytes[i] as usize;
                let b1 = bytes[i+1] as usize;
                let b2 = bytes[i+2] as usize;
                s.push(TABLE[(b0 >> 2)] as char);
                s.push(TABLE[((b0 & 3) << 4) | (b1 >> 4)] as char);
                s.push(TABLE[((b1 & 0xf) << 2) | (b2 >> 6)] as char);
                s.push(TABLE[b2 & 0x3f] as char);
                i += 3;
            }
            if i < bytes.len() {
                let b0 = bytes[i] as usize;
                s.push(TABLE[(b0 >> 2)] as char);
                if i + 1 < bytes.len() {
                    let b1 = bytes[i+1] as usize;
                    s.push(TABLE[((b0 & 3) << 4) | (b1 >> 4)] as char);
                    s.push(TABLE[((b1 & 0xf) << 2)] as char);
                } else {
                    s.push(TABLE[((b0 & 3) << 4)] as char);
                    let _ = write!(s, "=");
                }
                let _ = write!(s, "=");
            }
            s
        };

        let script = format!("start powershell -NoExit -EncodedCommand {}", b64);

        Command::new("cmd")
            .args(["/c", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;

        return Ok("已打开 PowerShell（OpenClaw 环境已注入）".to_string());
        }
        #[cfg(not(windows))]
        return Err("平台不匹配".to_string());
    } else if platform::is_macos() {
        let env_vars: Vec<String> = if let Ok(content) = std::fs::read_to_string(&env_file) {
            content.lines()
                .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                .map(|l| {
                    let l = l.trim().strip_prefix("export ").unwrap_or(l.trim());
                    format!("export {}", l)
                })
                .collect()
        } else { vec![] };

        let mut lines = vec![
            "#!/bin/bash".to_string(),
            // source user rc first so our PATH wins
            r#"[ -f "$HOME/.zshrc" ] && source "$HOME/.zshrc" 2>/dev/null || true"#.to_string(),
            r#"[ -f "$HOME/.bashrc" ] && source "$HOME/.bashrc" 2>/dev/null || true"#.to_string(),
            // prepend our paths so they take priority
            format!("export PATH=\"{}:$PATH\"", format!("{}/node:{}/bin", config_dir, config_dir)),
        ];
        lines.extend(env_vars);
        lines.push("echo -e '\\033[32mOpenClaw 环境已就绪\\033[0m'".to_string());
        lines.push(format!("echo '配置目录: {}'", config_dir));
        lines.push("exec \"$SHELL\"".to_string());

        let script_path = "/tmp/openclaw_env_terminal.command";
        std::fs::write(script_path, lines.join("\n"))
            .map_err(|e| format!("创建脚本失败: {}", e))?;
        Command::new("chmod").args(["+x", script_path]).output().ok();
        Command::new("open").arg(script_path).spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;

        Ok("已打开终端（OpenClaw 环境已注入）".to_string())
    } else {
        // Linux
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

        let env_vars: Vec<String> = if let Ok(content) = std::fs::read_to_string(&env_file) {
            content.lines()
                .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                .map(|l| {
                    let l = l.trim().strip_prefix("export ").unwrap_or(l.trim());
                    format!("export {}", l)
                })
                .collect()
        } else { vec![] };

        let mut lines = vec![
            "#!/bin/bash".to_string(),
            r#"[ -f "$HOME/.bashrc" ] && source "$HOME/.bashrc" 2>/dev/null || true"#.to_string(),
            r#"[ -f "$HOME/.zshrc" ] && source "$HOME/.zshrc" 2>/dev/null || true"#.to_string(),
            format!("export PATH=\"{}/node:{}/bin:$PATH\"", config_dir, config_dir),
        ];
        lines.extend(env_vars);
        lines.push("echo -e '\\033[32mOpenClaw 环境已就绪\\033[0m'".to_string());
        lines.push(format!("echo '配置目录: {}'", config_dir));
        lines.push(format!("exec \"{}\"", shell));

        let script_path = "/tmp/openclaw_env_terminal.sh";
        std::fs::write(script_path, lines.join("\n"))
            .map_err(|e| format!("创建脚本失败: {}", e))?;
        Command::new("chmod").args(["+x", script_path]).output().ok();

        let terminals: &[(&str, Vec<&str>)] = &[
            ("gnome-terminal", vec!["--"]),
            ("xterm", vec!["-e"]),
            ("konsole", vec!["-e"]),
            ("xfce4-terminal", vec!["-e"]),
            ("tilix", vec!["-e"]),
            ("alacritty", vec!["-e"]),
            ("kitty", vec![]),
        ];
        for (term, args) in terminals {
            let mut cmd = Command::new(term);
            cmd.args(args.iter().map(|s| *s));
            cmd.arg(script_path);
            if cmd.spawn().is_ok() {
                return Ok(format!("已打开 {}（OpenClaw 环境已注入）", term));
            }
        }
        Err(format!(
            "未找到可用终端，请手动运行：\nsource {}\n然后执行 {}",
            script_path, shell
        ))
    }
}
