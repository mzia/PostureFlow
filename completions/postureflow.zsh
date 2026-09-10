#compdef postureflow

_postureflow() {
    local -a options=(
        '--home[Activate Home / Streaming / Gaming mode]'
        '--work[Activate Work / Office / Corporate VPN mode]'
        '--dev[Activate Developer / Coding mode]'
        '-t[Activate Hardened Travel / Lockdown mode]'
        '--travel[Activate Hardened Travel / Lockdown mode]'
        '-s[Legacy alias for Travel mode]'
        '--secure[Legacy alias for Travel mode]'
        '-i[Display current context posture and open ports]'
        '--status[Display current context posture and open ports]'
        '-r[Reset all firewall, sysctl, and power settings to Pop!_OS factory defaults]'
        '--reset[Reset all firewall, sysctl, and power settings to Pop!_OS factory defaults]'
        '--default[Alias to reset system to factory defaults]'
        '-v[Show version information]'
        '--version[Show version information]'
        '-h[Show help menu]'
        '--help[Show help menu]'
    )
    _describe -t options 'postureflow commands' options
}

_postureflow "$@"
