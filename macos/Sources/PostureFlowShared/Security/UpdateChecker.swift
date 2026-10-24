import Foundation

/// Report describing local repository and remote `origin` update status across machines.
public struct UpdateCheckReport: Codable, Equatable, Sendable {
    public var currentVersion: String
    public var repoPath: String?
    public var branch: String
    public var localCommit: String?
    public var remoteCommit: String?
    public var commitsBehind: Int
    public var commitsAhead: Int
    public var dirtyFilesCount: Int
    public var incomingCommits: [String]
    public var updateAvailable: Bool
    public var summary: String

    public init(
        currentVersion: String = "1.0.1",
        repoPath: String? = nil,
        branch: String = "main",
        localCommit: String? = nil,
        remoteCommit: String? = nil,
        commitsBehind: Int = 0,
        commitsAhead: Int = 0,
        dirtyFilesCount: Int = 0,
        incomingCommits: [String] = [],
        updateAvailable: Bool = false,
        summary: String = ""
    ) {
        self.currentVersion = currentVersion
        self.repoPath = repoPath
        self.branch = branch
        self.localCommit = localCommit
        self.remoteCommit = remoteCommit
        self.commitsBehind = commitsBehind
        self.commitsAhead = commitsAhead
        self.dirtyFilesCount = dirtyFilesCount
        self.incomingCommits = incomingCommits
        self.updateAvailable = updateAvailable
        self.summary = summary
    }
}

/// Cross-machine git & GitHub update checker for PostureFlow on macOS.
public enum UpdateChecker {
    public static let appVersion = "1.0.1"
    public static let githubRepoURL = "https://github.com/mzia/PostureFlow"

    /// Locates the PostureFlow git checkout on this Mac.
    public static func locateGitRepo() -> String? {
        let fm = FileManager.default
        var candidates: [String] = []

        if let envRepo = ProcessInfo.processInfo.environment["POSTUREFLOW_REPO"], !envRepo.isEmpty {
            candidates.append(envRepo)
        }

        let cwd = fm.currentDirectoryPath
        candidates.append(cwd)
        candidates.append((cwd as NSString).deletingLastPathComponent)

        if let exeURL = Bundle.main.executableURL {
            var dir = exeURL.deletingLastPathComponent()
            for _ in 0..<6 {
                candidates.append(dir.path)
                dir = dir.deletingLastPathComponent()
            }
        }

        let home = fm.homeDirectoryForCurrentUser.path
        candidates.append("\(home)/workspace/PostureFlow")
        candidates.append("\(home)/PostureFlow")
        candidates.append("\(home)/Projects/PostureFlow")
        candidates.append("\(home)/Developer/PostureFlow")

        for path in candidates {
            let gitDir = (path as NSString).appendingPathComponent(".git")
            let cargoFile = (path as NSString).appendingPathComponent("Cargo.toml")
            if fm.fileExists(atPath: gitDir) && fm.fileExists(atPath: cargoFile) {
                return path
            }
        }

        return nil
    }

    @discardableResult
    private static func runGit(repo: String, args: [String]) -> (success: Bool, output: String, error: String) {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = ["-C", repo] + args

        let outPipe = Pipe()
        let errPipe = Pipe()
        process.standardOutput = outPipe
        process.standardError = errPipe

        do {
            try process.run()
            process.waitUntilExit()
            let outData = outPipe.fileHandleForReading.readDataToEndOfFile()
            let errData = errPipe.fileHandleForReading.readDataToEndOfFile()
            let outStr = String(data: outData, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            let errStr = String(data: errData, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            return (process.terminationStatus == 0, outStr, errStr)
        } catch {
            return (false, "", error.localizedDescription)
        }
    }

    /// Checks `origin/<branch>` for new cross-machine commits (e.g. pushed from Pop!_OS Framework).
    public static func checkForUpdates(fetchRemote: Bool = true) -> UpdateCheckReport {
        var report = UpdateCheckReport(currentVersion: appVersion, branch: "main")

        guard let repo = locateGitRepo() else {
            report.summary = "PostureFlow v\(appVersion) (No local git repository detected)"
            return report
        }

        report.repoPath = repo

        let branchRes = runGit(repo: repo, args: ["rev-parse", "--abbrev-ref", "HEAD"])
        if branchRes.success && !branchRes.output.isEmpty && branchRes.output != "HEAD" {
            report.branch = branchRes.output
        }

        if fetchRemote {
            _ = runGit(repo: repo, args: ["fetch", "origin", "--prune", "--quiet"])
        }

        let remoteRef = "origin/\(report.branch)"

        let localRes = runGit(repo: repo, args: ["log", "-1", "--format=%h (%cr) %s", "HEAD"])
        if localRes.success && !localRes.output.isEmpty {
            report.localCommit = localRes.output
        }

        let remoteRes = runGit(repo: repo, args: ["log", "-1", "--format=%h (%cr) %s", remoteRef])
        if remoteRes.success && !remoteRes.output.isEmpty {
            report.remoteCommit = remoteRes.output
        }

        let behindRes = runGit(repo: repo, args: ["rev-list", "--count", "HEAD..\(remoteRef)"])
        if behindRes.success, let count = Int(behindRes.output) {
            report.commitsBehind = count
        }

        let aheadRes = runGit(repo: repo, args: ["rev-list", "--count", "\(remoteRef)..HEAD"])
        if aheadRes.success, let count = Int(aheadRes.output) {
            report.commitsAhead = count
        }

        let statusRes = runGit(repo: repo, args: ["status", "--porcelain"])
        if statusRes.success && !statusRes.output.isEmpty {
            report.dirtyFilesCount = statusRes.output
                .components(separatedBy: .newlines)
                .filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }
                .count
        }

        if report.commitsBehind > 0 {
            let logRes = runGit(repo: repo, args: ["log", "HEAD..\(remoteRef)", "--oneline", "-n", "10"])
            if logRes.success && !logRes.output.isEmpty {
                report.incomingCommits = logRes.output
                    .components(separatedBy: .newlines)
                    .map { $0.trimmingCharacters(in: .whitespaces) }
                    .filter { !$0.isEmpty }
            }
        }

        report.updateAvailable = report.commitsBehind > 0

        if report.updateAvailable {
            let dirtySuffix = report.dirtyFilesCount > 0 ? " (\(report.dirtyFilesCount) local uncommitted)" : ""
            report.summary = "Update available: \(report.commitsBehind) new commit(s) on \(remoteRef)\(dirtySuffix)"
        } else if report.commitsAhead > 0 {
            let dirtySuffix = report.dirtyFilesCount > 0 ? ", \(report.dirtyFilesCount) uncommitted" : ""
            report.summary = "Up to date (\(report.commitsAhead) local commit(s) ahead\(dirtySuffix))"
        } else {
            let shaRes = runGit(repo: repo, args: ["rev-parse", "--short", "HEAD"])
            let shortSHA = shaRes.success && !shaRes.output.isEmpty ? shaRes.output : "HEAD"
            let dirtySuffix = report.dirtyFilesCount > 0 ? ", \(report.dirtyFilesCount) uncommitted" : ""
            report.summary = "Up to date (v\(appVersion), \(shortSHA) on \(report.branch)\(dirtySuffix))"
        }

        return report
    }

    /// Fast-forwards the local git checkout from `origin/<branch>`, safely stashing and restoring
    /// any uncommitted tracked modifications.
    public static func applyUpdate() -> (success: Bool, message: String) {
        guard let repo = locateGitRepo() else {
            return (false, "Could not locate local PostureFlow git repository.")
        }

        let report = checkForUpdates(fetchRemote: true)
        if !report.updateAvailable {
            return (true, "Already up to date: \(report.summary)")
        }

        let remoteRef = "origin/\(report.branch)"
        let trackedStatus = runGit(repo: repo, args: ["status", "--porcelain", "-uno"])
        let hasTrackedChanges = trackedStatus.success && !trackedStatus.output.isEmpty

        if hasTrackedChanges {
            let stashRes = runGit(repo: repo, args: ["stash", "push", "-m", "postureflow-auto-update"])
            if !stashRes.success {
                return (false, "Failed to stash local changes: \(stashRes.error)")
            }
        }

        let mergeRes = runGit(repo: repo, args: ["merge", "--ff-only", remoteRef])
        if !mergeRes.success {
            if hasTrackedChanges {
                _ = runGit(repo: repo, args: ["stash", "pop"])
            }
            return (false, "Fast-forward merge from \(remoteRef) failed: \(mergeRes.error)")
        }

        if hasTrackedChanges {
            let popRes = runGit(repo: repo, args: ["stash", "pop"])
            if !popRes.success {
                return (false, "Fast-forwarded to \(remoteRef), but 'git stash pop' had conflicts in \(repo).")
            }
        }

        let newHead = runGit(repo: repo, args: ["log", "-1", "--format=%h (%s)", "HEAD"]).output
        return (true, "Updated \(report.commitsBehind) commit(s) from \(remoteRef) -> \(newHead). Run ./install.sh to rebuild.")
    }
}
