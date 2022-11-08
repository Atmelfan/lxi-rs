#!/bin/bash
echo "Updating readmes..."
for manifest in */Cargo.toml; do
    crate_dir="$(dirname $manifest)";

    # Use root template if crate does not have its own
    template=""
    if [ ! -f "$crate_dir/README.tpl" ]; then
        template="-t ../README.tpl $args";
    fi

    # Update readme
    echo "Updating $crate_dir with '$template'"
    cargo readme -r "$crate_dir" $template "$@" > "$crate_dir/README.md"
done