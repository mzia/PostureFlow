#compdef pop-profile

_pop_profile() {
    local -a options=(
        '--home[Activate Home / Streaming / Gaming mode]'
        '--work[Activate Work / Office / Corporate VPN mode]'
        '--dev[Activate Developer / Coding mode]'
        '--secure[Activate Hardened Travel / Lockdown mode]'
        '-i[Display current context posture and open ports]'
        '--status[Display current context posture and open ports]'
        '--test[Run automated anti-lockout safety test suite]'
        '--reset[Reset all firewall and sysctl settings to Pop!_OS factory defaults]'
        '-v[Show version information]'
        '--version[Show version information]'
        '-h[Show help menu]'
        '--help[Show help menu]'
    )
    _describe -t options 'pop-profile commands' options
}

_pop_profile "$@"
