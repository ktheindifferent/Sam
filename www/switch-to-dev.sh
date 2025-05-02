#!/bin/bash

for file in *.html apps/*/*.html; do
  # Find all /assets/dist/css/*.min.css references (with subdirs)
  grep -o '/assets/dist/css/[a-zA-Z0-9/_-]*\.min\.css' "$file" | while read -r asset; do
    dev_path="${asset/assets\/dist/assets}"
    base="${dev_path%.min.css}"
    # If .min.css exists, use it; else use .css (preserve subdirs)
    if [ -f ".${base}.min.css" ]; then
      replacement="${base}.min.css"
    else
      replacement="${base}.css"
    fi
    # Replace in file
    sed -i '' "s#${asset}#${replacement}#g" "$file"
  done

  # Find all /assets/dist/js/*.min.js references (with subdirs)
  grep -o '/assets/dist/js/[a-zA-Z0-9/_-]*\.min\.js' "$file" | while read -r asset; do
    dev_path="${asset/assets\/dist/assets}"
    base="${dev_path%.min.js}"
    if [ -f ".${base}.min.js" ]; then
      replacement="${base}.min.js"
    else
      replacement="${base}.js"
    fi
    sed -i '' "s#${asset}#${replacement}#g" "$file"
  done
done

echo "Switched HTML asset references to development /assets/ paths, preserving subdirectories."