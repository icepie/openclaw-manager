use crate::utils::{platform, shell};
use serde::{Deserialize, Serialize};
use tauri::{command, Emitter, Manager};
use log::{info, warn, error, debug};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    if platform::is_windows() {
        // Windows: 先尝试直接调用（如果 PATH 已更新）
        if let Ok(v) = shell::run_cmd_output("node --version") {
            let version = v.trim().to_string();
            if !version.is_empty() && version.starts_with('v') {
                info!("[环境检查] 通过 PATH 找到 Node.js: {}", version);
                return Some(version);
            }
        }
        
        // Windows: 检查常见的安装路径
        let possible_paths = get_windows_node_paths();
        for path in possible_paths {
            if std::path::Path::new(&path).exists() {
                // 使用完整路径执行
                let cmd = format!("\"{}\" --version", path);
                if let Ok(output) = shell::run_cmd_output(&cmd) {
                    let version = output.trim().to_string();
                    if !version.is_empty() && version.starts_with('v') {
                        info!("[环境检查] 在 {} 找到 Node.js: {}", path, version);
                        return Some(version);
                    }
                }
            }
        }
        
        None
    } else {
        // 先尝试直接调用
        if let Ok(v) = shell::run_command_output("node", &["--version"]) {
            return Some(v.trim().to_string());
        }
        
        // 检测常见的 Node.js 安装路径（macOS/Linux）
        let possible_paths = get_unix_node_paths();
        for path in possible_paths {
            if std::path::Path::new(&path).exists() {
                if let Ok(output) = shell::run_command_output(&path, &["--version"]) {
                    info!("[环境检查] 在 {} 找到 Node.js: {}", path, output.trim());
                    return Some(output.trim().to_string());
                }
            }
        }
        
        // 尝试通过 shell 加载用户环境来检测
        if let Ok(output) = shell::run_bash_output("source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null; node --version 2>/dev/null") {
            if !output.is_empty() && output.starts_with('v') {
                info!("[环境检查] 通过用户 shell 找到 Node.js: {}", output.trim());
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
        &["bin/openclaw.cmd", "bin/openclaw.exe"]
    } else {
        &["bin/openclaw"]
    };
    candidates.iter().any(|rel| prefix.join(rel).exists())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = fs::metadata(&src_path) {
                    let _ = fs::set_permissions(&dst_path, meta.permissions());
                }
            }
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

fn install_openclaw_from_bundle_dir(bundle_dir: &PathBuf, install_dir: Option<&Path>) -> Result<bool, String> {
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
    fs::create_dir_all(&prefix).map_err(|e| e.to_string())?;

    let prepared_prefix = bundle_dir.join("prefix");
    if prepared_prefix.exists() {
        info!("[离线安装] 从 bundled prefix snapshot 安装...");
        copy_dir_recursive(&prepared_prefix, &prefix)?;
        // 同时把 bundled node 复制到 prefix/node/，供 openclaw 运行时使用
        copy_bundled_node_to_prefix(bundle_dir, &prefix)?;
        if prefix_has_openclaw_binary(&prefix) {
            info!("[离线安装] ✓ bundled prefix 安装完成");
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
    let output = Command::new(&node_bin)
        .arg(&npm_cli)
        .arg("install")
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
        copy_bundled_node_to_prefix(bundle_dir, &prefix)?;
        if prefix_has_openclaw_binary(&prefix) {
            info!("[离线安装] ✓ npm 离线安装完成");
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
    match install_openclaw_from_bundle_dir(&bundle_dir, install_dir) {
        Ok(result) => Some(result),
        Err(e) => {
            warn!("[离线安装] 失败: {}", e);
            None
        }
    }
}

// ── bundle 下载安装 ────────────────────────────────────────────────────────────

/// 返回当前平台对应的 bundle 默认下载 URL
#[command]
pub fn get_bundle_download_url() -> String {
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
        "https://github.com/icepie/openclaw-manager/releases/latest/download/openclaw-bundle-{}-{}.{}",
        os, arch, ext
    )
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
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let o = Command::new("tar")
        .arg("-xzf").arg(archive)
        .arg("-C").arg(dest)
        .output()
        .map_err(|e| format!("tar 解压失败: {}", e))?;
    if o.status.success() {
        return Ok(());
    }
    // Windows: tar 可能不支持 -z，尝试不带 -z
    let o2 = Command::new("tar")
        .arg("-xf").arg(archive)
        .arg("-C").arg(dest)
        .output()
        .map_err(|e| format!("tar 解压失败: {}", e))?;
    if o2.status.success() {
        return Ok(());
    }
    Err(format!("tar 解压失败: {}", String::from_utf8_lossy(&o2.stderr).trim()))
}

fn extract_zip(archive: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    // Windows 内置 Expand-Archive (PowerShell)
    let o = Command::new("powershell")
        .args([
            "-NoProfile", "-NonInteractive", "-Command",
            &format!(
                "Expand-Archive -Force -Path '{}' -DestinationPath '{}'",
                archive.display(), dest.display()
            ),
        ])
        .output()
        .map_err(|e| format!("zip 解压失败: {}", e))?;
    if o.status.success() {
        return Ok(());
    }
    Err(format!("zip 解压失败: {}", String::from_utf8_lossy(&o.stderr).trim()))
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
    match install_openclaw_from_bundle_dir(&bundle_dir, install_dir) {
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
pub async fn install_openclaw(app: tauri::AppHandle, bundle_url: Option<String>, install_dir: Option<String>) -> Result<InstallResult, String> {
    info!("[安装OpenClaw] 开始安装 OpenClaw...");

    let dir: Option<PathBuf> = install_dir.map(PathBuf::from);
    let dir_ref: Option<&Path> = dir.as_deref();

    // 1. 优先本地 bundle（打包进 app 的）
    if let Some(true) = try_install_openclaw_offline(&app, dir_ref) {
        info!("[安装OpenClaw] ✓ 本地 bundle 安装成功");
        return Ok(InstallResult {
            success: true,
            message: "OpenClaw 安装成功！".to_string(),
            error: None,
        });
    }

    // 2. 本地没有，从指定 URL（或默认）下载 bundle 安装
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
    
    // 设置 gateway mode 为 local
    info!("[初始化配置] 执行: openclaw config set gateway.mode local");
    let result = shell::run_openclaw(&["config", "set", "gateway.mode", "local"]);
    
    match result {
        Ok(output) => {
            info!("[初始化配置] ✓ 配置初始化成功");
            debug!("[初始化配置] 命令输出: {}", output);
            Ok(InstallResult {
                success: true,
                message: "配置初始化成功！".to_string(),
                error: None,
            })
        },
        Err(e) => {
            error!("[初始化配置] ✗ 配置初始化失败: {}", e);
            Ok(InstallResult {
                success: false,
                message: "配置初始化失败".to_string(),
                error: Some(e),
            })
        },
    }
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

/// 卸载 OpenClaw
#[command]
pub async fn uninstall_openclaw() -> Result<InstallResult, String> {
    info!("[卸载OpenClaw] 开始卸载 OpenClaw...");
    let os = platform::get_os();
    info!("[卸载OpenClaw] 检测到操作系统: {}", os);
    
    // 先停止服务
    info!("[卸载OpenClaw] 尝试停止服务...");
    let _ = shell::run_openclaw(&["gateway", "stop"]);
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    let result = match os.as_str() {
        "windows" => {
            info!("[卸载OpenClaw] 使用 Windows 卸载方式...");
            uninstall_openclaw_windows().await
        },
        _ => {
            info!("[卸载OpenClaw] 使用 Unix 卸载方式 (npm)...");
            uninstall_openclaw_unix().await
        },
    };
    
    match &result {
        Ok(r) if r.success => info!("[卸载OpenClaw] ✓ 卸载成功"),
        Ok(r) => warn!("[卸载OpenClaw] ✗ 卸载失败: {}", r.message),
        Err(e) => error!("[卸载OpenClaw] ✗ 卸载错误: {}", e),
    }
    
    result
}

/// Windows 卸载 OpenClaw
async fn uninstall_openclaw_windows() -> Result<InstallResult, String> {
    // 使用 cmd.exe 执行 npm uninstall，避免 PowerShell 执行策略问题
    info!("[卸载OpenClaw] 执行 npm uninstall -g openclaw...");
    
    match shell::run_cmd_output("npm uninstall -g openclaw") {
        Ok(output) => {
            info!("[卸载OpenClaw] npm 输出: {}", output);
            
            // 验证卸载是否成功
            std::thread::sleep(std::time::Duration::from_millis(500));
            if get_openclaw_version().is_none() {
                Ok(InstallResult {
                    success: true,
                    message: "OpenClaw 已成功卸载！".to_string(),
                    error: None,
                })
            } else {
                Ok(InstallResult {
                    success: false,
                    message: "卸载命令已执行，但 OpenClaw 仍然存在，请尝试手动卸载".to_string(),
                    error: Some(output),
                })
            }
        }
        Err(e) => {
            warn!("[卸载OpenClaw] npm uninstall 失败: {}", e);
            Ok(InstallResult {
                success: false,
                message: "OpenClaw 卸载失败".to_string(),
                error: Some(e),
            })
        }
    }
}

/// Unix 系统卸载 OpenClaw
async fn uninstall_openclaw_unix() -> Result<InstallResult, String> {
    let script = r#"
echo "卸载 OpenClaw..."
npm uninstall -g openclaw

# 验证卸载
if command -v openclaw &> /dev/null; then
    echo "警告：openclaw 命令仍然存在"
    exit 1
else
    echo "OpenClaw 已成功卸载"
    exit 0
fi
"#;
    
    match shell::run_bash_output(script) {
        Ok(output) => Ok(InstallResult {
            success: true,
            message: format!("OpenClaw 已成功卸载！{}", output),
            error: None,
        }),
        Err(e) => Ok(InstallResult {
            success: false,
            message: "OpenClaw 卸载失败".to_string(),
            error: Some(e),
        }),
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

/// 打开一个已注入 ~/.openclaw/bin 到 PATH 的终端
#[command]
pub async fn open_env_terminal() -> Result<String, String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "~".to_string());
    let openclaw_bin = format!("{}/.openclaw/bin", home);

    if platform::is_windows() {
        // PowerShell: 在新窗口中注入 PATH 并保持打开
        let script = format!(
            r#"Start-Process powershell -ArgumentList '-NoExit', '-Command', '$env:PATH = "{bin};$env:PATH"; Write-Host "OpenClaw PATH 已注入: {bin}" -ForegroundColor Green; Write-Host "当前 PATH 包含: $(($env:PATH -split ";") -join "`n")" -ForegroundColor Gray'"#,
            bin = openclaw_bin.replace('/', "\\")
        );
        shell::run_powershell_output(&script)?;
        Ok("已打开终端（PATH 已注入）".to_string())
    } else if platform::is_macos() {
        let script_content = format!(
            "#!/bin/sh\nexport PATH=\"{bin}:$PATH\"\nexec \"$SHELL\" -l\n",
            bin = openclaw_bin
        );
        let script_path = "/tmp/openclaw_env_terminal.command";
        std::fs::write(script_path, script_content)
            .map_err(|e| format!("创建脚本失败: {}", e))?;
        Command::new("chmod").args(["+x", script_path]).output()
            .map_err(|e| format!("设置权限失败: {}", e))?;
        Command::new("open").arg(script_path).spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;
        Ok("已打开终端（PATH 已注入）".to_string())
    } else {
        // Linux: 尝试常见终端模拟器
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let env_arg = format!("PATH={}:$PATH", openclaw_bin);
        let terminals: &[(&str, &[&str])] = &[
            ("gnome-terminal", &["--"]),
            ("xterm", &["-e"]),
            ("konsole", &["-e"]),
            ("xfce4-terminal", &["-e"]),
        ];
        for (term, args) in terminals {
            let mut cmd = Command::new(term);
            cmd.args(*args).arg(&shell);
            cmd.env("PATH", format!("{}:{}", openclaw_bin,
                std::env::var("PATH").unwrap_or_default()));
            if cmd.spawn().is_ok() {
                return Ok(format!("已打开 {} （PATH 已注入）", term));
            }
        }
        // fallback: 输出提示命令
        Err(format!(
            "未找到可用终端，请手动运行：\nexport PATH=\"{}:$PATH\"\n然后执行 {} -l",
            openclaw_bin, shell
        ))
    }
}
