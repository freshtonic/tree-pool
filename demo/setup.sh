#!/bin/bash
# Fake environment for demo recording.
# cd to a temp dir so files created during the demo don't leak into the repo.
cd "$(mktemp -d)"
export PS1='$ '

tp() {
    case "$1 $2" in
        "init ")
            echo "created /Users/you/projects/acme-api/tree-pool.toml" >&2
            ;;
        "get feature/billing")
            echo "fetching origin..." >&2
            echo "created tree: ~/.tree-pool/acme-api-a1b2c3/1/acme-api" >&2
            echo "on branch: feature/billing" >&2
            bash --norc --noprofile -i
            ;;
        "get fix/auth-timeout")
            echo "fetching origin..." >&2
            echo "created tree: ~/.tree-pool/acme-api-a1b2c3/2/acme-api" >&2
            echo "on branch: fix/auth-timeout" >&2
            bash --norc --noprofile -i
            ;;
        "status ")
            printf "   1  %-11s  %-20s  %s\n" "available" "feature/billing" "~/.tree-pool/acme-api-a1b2c3/1/acme-api"
            printf "   2  %-11s  %-20s  %s\n" "available" "fix/auth-timeout" "~/.tree-pool/acme-api-a1b2c3/2/acme-api"
            ;;
    esac
}

git() {
    :
}

export -f tp git
export PS1
