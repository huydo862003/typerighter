use std::fs;
use zed_extension_api::{self as zed, LanguageServerId, Result, settings::LspSettings};

struct TypedownExtension {
  cached_binary_path: Option<String>,
}

impl TypedownExtension {
  /// Artifact naming: typedown-lsp-{version}-{os}-{arch}[.exe]
  fn os_arch() -> Result<(&'static str, &'static str)> {
    let (platform, arch) = zed::current_platform();
    let os = match platform {
      zed::Os::Mac => "darwin",
      zed::Os::Linux => "linux",
      zed::Os::Windows => "windows",
    };
    let arch = match arch {
      zed::Architecture::Aarch64 => "aarch64",
      zed::Architecture::X8664 => "x86_64",
      zed::Architecture::X86 => return Err("unsupported architecture: x86".into()),
    };
    Ok((os, arch))
  }

  fn resolve_binary(
    &mut self,
    language_server_id: &LanguageServerId,
    worktree: &zed::Worktree,
  ) -> Result<String> {
    // 1. User-configured path
    if let Some(path) = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
      .ok()
      .and_then(|s| s.binary)
      .and_then(|b| b.path)
    {
      return Ok(path);
    }

    // 2. Dev build (path embedded at compile time by build.rs when extension.toml has file:// URLs)
    if let Some(dev_path) = option_env!("TYPEDOWN_DEV_LSP_PATH") {
      return Ok(dev_path.to_string());
    }

    // 3. Binary on $PATH
    if let Some(path) = worktree.which("typedown-lsp") {
      return Ok(path);
    }

    // 4. Cached download
    if let Some(path) = &self.cached_binary_path
      && fs::metadata(path).is_ok_and(|stat| stat.is_file())
    {
      return Ok(path.clone());
    }

    // 5. Download from GitHub releases
    zed::set_language_server_installation_status(
      language_server_id,
      &zed::LanguageServerInstallationStatus::CheckingForUpdate,
    );

    let release = zed::latest_github_release(
      "huydo862003/typerighter",
      zed::GithubReleaseOptions {
        require_assets: true,
        pre_release: false,
      },
    )?;

    let (os, arch) = Self::os_arch()?;
    let ext = if os == "windows" { ".exe" } else { "" };
    // release.version is the tag name (e.g. "v0.1.0"), strip the "v" prefix
    let version = release
      .version
      .strip_prefix('v')
      .unwrap_or(&release.version);
    let asset_name = format!("typedown-lsp-{version}-{os}-{arch}{ext}");

    let asset = release
      .assets
      .iter()
      .find(|a| a.name == asset_name)
      .ok_or_else(|| format!("no release asset matching {asset_name:?}"))?;

    let version_dir = format!("typedown-lsp-{version}");
    let binary_path = format!("{version_dir}/typedown-lsp{ext}");

    if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
      zed::set_language_server_installation_status(
        language_server_id,
        &zed::LanguageServerInstallationStatus::Downloading,
      );

      zed::download_file(
        &asset.download_url,
        &binary_path,
        zed::DownloadedFileType::Uncompressed,
      )
      .map_err(|err| format!("failed to download typedown-lsp: {err}"))?;

      zed::make_file_executable(&binary_path)
        .map_err(|err| format!("failed to make typedown-lsp executable: {err}"))?;

      // Clean up old versions
      if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
          let name = entry.file_name();
          if name
            .to_str()
            .is_some_and(|n| n.starts_with("typedown-lsp-") && n != version_dir)
          {
            fs::remove_dir_all(entry.path()).ok();
          }
        }
      }
    }

    self.cached_binary_path = Some(binary_path.clone());
    Ok(binary_path)
  }
}

impl zed::Extension for TypedownExtension {
  fn new() -> Self {
    Self {
      cached_binary_path: None,
    }
  }

  fn language_server_command(
    &mut self,
    language_server_id: &LanguageServerId,
    worktree: &zed::Worktree,
  ) -> Result<zed::Command> {
    let binary = self.resolve_binary(language_server_id, worktree)?;
    Ok(zed::Command {
      command: binary,
      args: vec!["--stdio".to_string()],
      env: vec![],
    })
  }
}

zed::register_extension!(TypedownExtension);
