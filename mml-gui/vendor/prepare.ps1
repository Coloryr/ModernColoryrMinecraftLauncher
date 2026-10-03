# 拉取 tauri-plugin-decoration 指定版本源码 + 应用本地 patch
#
# 为什么这样组织
# ------------------------------------------------------------------
# 我们需要改 tauri-plugin-decoration 的源码（让它接受应用提供的窗口按钮矩形，
# 这样既能保住 M²L 自己的标题栏外观、又保留 Win11 贴靠布局）。
#
# 仓库里**不存**第三方源码副本：只存 patch，构建前由本脚本按**固定 commit**
# 从上游克隆、再打 patch。这样：
#   - 仓库干净，没有大段第三方源码；
#   - 改动一目了然（就是 patches/ 里那几个文件）；
#   - 上游升级只需改 $Ref 并重放 patch，冲突处有 `M²L 本地修改` 注释可对照。
#
# 用法：
#   pwsh -File mml-gui/vendor/prepare.ps1            # 幂等，已存在则跳过
#   pwsh -File mml-gui/vendor/prepare.ps1 -Force     # 强制重新拉取
#
# 注意：产物放在 target/vendor 下（已被 git 忽略）。cargo 在**解析依赖图时**
# 就要求 path 依赖的 Cargo.toml 存在，所以这个脚本必须在 cargo 之前跑 ——
# 各构建入口（build-*.bat / npm 脚本）都已接好。

param(
    [switch]$Force
)

$ErrorActionPreference = "Stop"

# 上游仓库与版本。$Ref 用 **commit**（不是 tag）：tag 可被移动，commit 不会
$Repo = "https://github.com/oovz/tauri-plugin-decoration.git"
$Ref  = "235272b40d9a7b840ad49944e9fef334fde251ec"   # = v3.0.5
$DirName = "tauri-plugin-decoration"

$vendorDir = $PSScriptRoot
# $PSScriptRoot = <repo>/mml-gui/vendor → 剥两层到仓库根
$repoRoot = Split-Path (Split-Path $vendorDir -Parent) -Parent
$patchDir = Join-Path $vendorDir "patches"
$outDir = Join-Path $repoRoot "target/vendor/$DirName"

if ((Test-Path (Join-Path $outDir "Cargo.toml")) -and -not $Force) {
    Write-Host "已存在，跳过：$outDir（要重新拉取加 -Force）"
    exit 0
}

if (Test-Path $outDir) {
    Remove-Item $outDir -Recurse -Force
}
New-Item -ItemType Directory -Force (Split-Path $outDir -Parent) | Out-Null

# 1. 浅克隆到指定 commit 并检出
#    先 init + fetch 指定 commit（比 clone 整个仓库再 checkout 快得多）
Write-Host "拉取上游 $Ref ..."
& git init --quiet $outDir
if ($LASTEXITCODE -ne 0) { throw "git init 失败" }

& git -C $outDir remote add origin $Repo
& git -C $outDir fetch --quiet --depth 1 origin $Ref
if ($LASTEXITCODE -ne 0) { throw "拉取上游失败（$Repo @ $Ref）" }

& git -C $outDir checkout --quiet FETCH_HEAD
if ($LASTEXITCODE -ne 0) { throw "检出 $Ref 失败" }

# 上游源码即可编译，.git 不需要留着（也避免它被当成独立仓库干扰外层 git）
Remove-Item (Join-Path $outDir ".git") -Recurse -Force -ErrorAction SilentlyContinue

# 2. 依次应用 patch
$patches = Get-ChildItem $patchDir -Filter *.patch -ErrorAction SilentlyContinue | Sort-Object Name
if (-not $patches) {
    Write-Host "没有 patch，产物即上游原样：$outDir"
    exit 0
}

foreach ($p in $patches) {
    # 逐个文件对照，确认 patch **真的**改了东西。
    # 之前踩过：git apply 报 "Skipped patch" 却退出码 0，什么都没改 —— 靠这个兜住。
    $touched = (Select-String -Path $p.FullName -Pattern '^\+\+\+ b/(.+)$' |
        ForEach-Object { $_.Matches[0].Groups[1].Value.Trim() })

    # 产物落在 target/ 下 —— 那属于**本仓库**。git apply 在仓库内会以仓库根为基准
    # 解析补丁路径（src/js/... → <repoRoot>/src/js/...，不存在），
    # 而且这种"找不到"是**静默返回 0**的（实测）。所以显式 --directory 指定基准。
    #
    # 不用 [System.IO.Path]::GetRelativePath —— 那是 .NET Core 的 API，
    # Windows PowerShell 5.1（.NET Framework）没有，会报 MethodNotFound。
    $rel = $outDir.Substring($repoRoot.Length).TrimStart('\', '/').Replace('\', '/')
    Push-Location $repoRoot
    try {
        $result = & git apply --whitespace=nowarn --directory=$rel $p.FullName 2>&1
        $code = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    if ($code -ne 0) {
        throw "patch 应用失败：$($p.Name)`n$($result -join "`n")"
    }

    foreach ($r in $touched) {
        $outFile = Join-Path $outDir $r
        if (-not (Test-Path $outFile)) {
            throw "patch $($p.Name) 声明的文件不存在于产物：$r"
        }
        # 补丁后文件里必须留下标记，否则说明补丁没真正落地
        $content = [System.IO.File]::ReadAllText($outFile)
        if ($content -notmatch 'M²L 本地修改') {
            throw "patch $($p.Name) 未落到 $r（找不到 M²L 本地修改 标记）"
        }
    }
    Write-Host "已应用：$($p.Name)（改动 $($touched.Count) 个文件）"
}

Write-Host "完成：$outDir"
