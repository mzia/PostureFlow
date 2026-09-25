import Foundation

/// Status of developer credentials on the host machine
public struct CredentialCloakStatus: Sendable, Codable, Equatable {
    public var isSSHAgentPurged: Bool
    public var isAWSCredentialsCloaked: Bool
    public var isPasswordManagerLocked: Bool
    public var lastActionTimestamp: Date?

    public init(
        isSSHAgentPurged: Bool = false,
        isAWSCredentialsCloaked: Bool = false,
        isPasswordManagerLocked: Bool = false,
        lastActionTimestamp: Date? = nil
    ) {
        self.isSSHAgentPurged = isSSHAgentPurged
        self.isAWSCredentialsCloaked = isAWSCredentialsCloaked
        self.isPasswordManagerLocked = isPasswordManagerLocked
        self.lastActionTimestamp = lastActionTimestamp
    }
}

/// Automatically cloaks developer secrets, purges decrypted SSH identities, and locks vaults
public struct CredentialCloakEngine: Sendable {
    public init() {}

    /// AWS Credentials file path (~/.aws/credentials)
    public static var awsCredentialsURL: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".aws", isDirectory: true)
            .appendingPathComponent("credentials")
    }

    /// Purges all active identities from the ssh-agent memory pool
    @discardableResult
    public static func purgeSSHAgent() -> Bool {
        let task = Process()
        task.launchPath = "/usr/bin/ssh-add"
        task.arguments = ["-D"]
        task.standardOutput = Pipe()
        task.standardError = Pipe()

        do {
            try task.run()
            task.waitUntilExit()
            return task.terminationStatus == 0
        } catch {
            return false
        }
    }

    /// Cloaks or restores filesystem permissions on ~/.aws/credentials
    @discardableResult
    public static func setAWSCredentialsCloaked(_ cloak: Bool) -> Bool {
        let url = awsCredentialsURL
        let path = url.path
        guard FileManager.default.fileExists(atPath: path) else {
            return true
        }

        // Posix permissions: 0o000 for cloaked (no access), 0o600 for uncloaked
        let targetPermissions: NSNumber = cloak ? 0o000 : 0o600
        do {
            try FileManager.default.setAttributes([.posixPermissions: targetPermissions], ofItemAtPath: path)
            return true
        } catch {
            return false
        }
    }

    /// Locks installed CLI password managers (1Password and Bitwarden)
    @discardableResult
    public static func lockPasswordManagers() -> Bool {
        var lockedAny = false

        // Check for 1Password CLI (op)
        let opPaths = ["/usr/local/bin/op", "/opt/homebrew/bin/op"]
        for p in opPaths where FileManager.default.fileExists(atPath: p) {
            let t = Process()
            t.launchPath = p
            t.arguments = ["signout"]
            _ = try? t.run()
            t.waitUntilExit()
            lockedAny = true
            break
        }

        // Check for Bitwarden CLI (bw)
        let bwPaths = ["/usr/local/bin/bw", "/opt/homebrew/bin/bw"]
        for p in bwPaths where FileManager.default.fileExists(atPath: p) {
            let t = Process()
            t.launchPath = p
            t.arguments = ["lock"]
            _ = try? t.run()
            t.waitUntilExit()
            lockedAny = true
            break
        }

        return lockedAny
    }

    /// Evaluates posture transition and performs appropriate cloaking/uncloaking
    public static func handlePostureChange(
        from previousMode: PostureMode,
        to newMode: PostureMode,
        config: CredentialCloakConfig
    ) -> CredentialCloakStatus {
        guard config.enabled else {
            return CredentialCloakStatus()
        }

        var status = CredentialCloakStatus()

        // Transitioning away from Dev to Work, Home, or Travel: Cloak!
        if previousMode == .dev && newMode != .dev {
            if config.purgeSSHAgentOnLeaveDev {
                status.isSSHAgentPurged = purgeSSHAgent()
            }
            if config.cloakAWSCredentials {
                status.isAWSCredentialsCloaked = setAWSCredentialsCloaked(true)
            }
            if config.lockPasswordManagers {
                status.isPasswordManagerLocked = lockPasswordManagers()
            }
            status.lastActionTimestamp = Date()
        }

        // Transitioning into Dev mode: Uncloak AWS credentials!
        if newMode == .dev {
            if config.cloakAWSCredentials {
                _ = setAWSCredentialsCloaked(false)
                status.isAWSCredentialsCloaked = false
            }
            status.lastActionTimestamp = Date()
        }

        return status
    }
}
