//! Version policy evaluated after native OS probing in the thin platform layer.

pub const MIN_WINDOWS_BUILD: u32 = 22_000;
pub const MIN_DOCUMENTED_BACKDROP_BUILD: u32 = 22_621;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Architecture {
    X64,
    Arm64,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowsHost {
    pub major: u32,
    pub build: u32,
    pub workstation: bool,
    pub native_architecture: Architecture,
    pub process_architecture: Architecture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformError {
    UnsupportedWindowsVersion,
    ServerEdition,
    UnsupportedProcessArchitecture,
    UnvalidatedArmEmulation,
}

impl WindowsHost {
    /// Windows 11 reports NT major version 10. Build and product type matter.
    /// This does not probe the OS or implement an installer requirement.
    pub fn validate(self) -> Result<(), PlatformError> {
        if self.major != 10 || self.build < MIN_WINDOWS_BUILD {
            return Err(PlatformError::UnsupportedWindowsVersion);
        }
        if !self.workstation {
            return Err(PlatformError::ServerEdition);
        }
        if self.process_architecture != Architecture::X64 {
            return Err(PlatformError::UnsupportedProcessArchitecture);
        }
        if self.native_architecture != Architecture::X64 {
            return Err(PlatformError::UnvalidatedArmEmulation);
        }
        Ok(())
    }

    /// Capability detection only: this does not claim a working Mica surface.
    pub fn supports_documented_backdrop(self) -> bool {
        self.validate().is_ok() && self.build >= MIN_DOCUMENTED_BACKDROP_BUILD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(build: u32) -> WindowsHost {
        WindowsHost {
            major: 10,
            build,
            workstation: true,
            native_architecture: Architecture::X64,
            process_architecture: Architecture::X64,
        }
    }

    #[test]
    fn rejects_windows_10_and_accepts_minimum_windows_11() {
        assert_eq!(
            host(19_045).validate(),
            Err(PlatformError::UnsupportedWindowsVersion)
        );
        assert_eq!(
            host(21_999).validate(),
            Err(PlatformError::UnsupportedWindowsVersion)
        );
        assert_eq!(host(22_000).validate(), Ok(()));
    }

    #[test]
    fn rejects_server_despite_high_build_number() {
        let mut server = host(26_100);
        server.workstation = false;
        assert_eq!(server.validate(), Err(PlatformError::ServerEdition));
    }

    #[test]
    fn does_not_claim_arm_emulation_is_validated() {
        let mut arm = host(26_100);
        arm.native_architecture = Architecture::Arm64;
        assert_eq!(
            arm.validate(),
            Err(PlatformError::UnvalidatedArmEmulation)
        );
        arm.process_architecture = Architecture::Arm64;
        assert_eq!(
            arm.validate(),
            Err(PlatformError::UnsupportedProcessArchitecture)
        );
    }

    #[test]
    fn backdrop_capability_has_its_own_minimum() {
        assert!(!host(22_000).supports_documented_backdrop());
        assert!(!host(22_620).supports_documented_backdrop());
        assert!(host(22_621).supports_documented_backdrop());
    }
}

