//! Version policy evaluated after native OS probing in the thin platform layer.

/// Windows 10 RTM. Accepted by policy; runtime compatibility still needs target-OS QA.
pub const MIN_WINDOWS_BUILD: u32 = 10_240;
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
    /// Windows 10 and Windows 11 both report NT major version 10.
    /// Build, product type, and architecture determine the supported policy.
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
    fn accepts_windows_10_and_windows_11_x64_workstations() {
        for build in [10_240, 14_393, 17_763, 19_045, 22_000, 22_621, 26_100] {
            assert_eq!(
                host(build).validate(),
                Ok(()),
                "supported policy build {build}"
            );
        }
    }

    #[test]
    fn rejects_pre_windows_10_builds_and_different_nt_major() {
        assert_eq!(
            host(MIN_WINDOWS_BUILD - 1).validate(),
            Err(PlatformError::UnsupportedWindowsVersion)
        );
        let mut older = host(26_100);
        older.major = 6;
        assert_eq!(
            older.validate(),
            Err(PlatformError::UnsupportedWindowsVersion)
        );
        older.major = 11;
        assert_eq!(
            older.validate(),
            Err(PlatformError::UnsupportedWindowsVersion)
        );
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
        assert_eq!(arm.validate(), Err(PlatformError::UnvalidatedArmEmulation));
        arm.process_architecture = Architecture::Arm64;
        assert_eq!(
            arm.validate(),
            Err(PlatformError::UnsupportedProcessArchitecture)
        );
    }

    #[test]
    fn backdrop_capability_has_its_own_minimum() {
        assert!(!host(10_240).supports_documented_backdrop());
        assert!(!host(19_045).supports_documented_backdrop());
        assert!(!host(22_000).supports_documented_backdrop());
        assert!(!host(22_620).supports_documented_backdrop());
        assert!(host(22_621).supports_documented_backdrop());
        let mut server = host(26_100);
        server.workstation = false;
        assert!(!server.supports_documented_backdrop());
        let mut emulated = host(26_100);
        emulated.native_architecture = Architecture::Arm64;
        assert!(!emulated.supports_documented_backdrop());
    }
}
