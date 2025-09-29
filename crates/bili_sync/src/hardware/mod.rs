pub mod gpu;
pub mod webgl;
pub mod fingerprint;

pub use fingerprint::HardwareFingerprint;
pub use gpu::{GpuInfo, GpuVendor};
pub use webgl::WebGLInfo;

#[derive(Debug, Clone)]
pub struct HardwareInfo {
    pub gpu: GpuInfo,
    pub webgl: WebGLInfo,
}

impl HardwareInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn nvidia_rtx4070ti() -> Self {
        Self {
            gpu: GpuInfo::nvidia_rtx4070ti(),
            webgl: WebGLInfo::chrome_default(),
        }
    }

    pub fn amd_rx7800xt() -> Self {
        Self {
            gpu: GpuInfo::amd_rx7800xt(),
            webgl: WebGLInfo::chrome_default(),
        }
    }

    pub fn intel_arc_a770() -> Self {
        Self {
            gpu: GpuInfo::intel_arc_a770(),
            webgl: WebGLInfo::chrome_default(),
        }
    }

    pub fn generate_dm_img_str(&self) -> String {
        self.webgl.to_dm_img_str()
    }

    pub fn generate_dm_cover_img_str(&self) -> String {
        self.gpu.to_dm_cover_img_str()
    }
}

impl Default for HardwareInfo {
    fn default() -> Self {
        Self::nvidia_rtx4070ti()
    }
}