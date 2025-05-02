#!/bin/bash

# Switch HTML asset references from development to production (dist) paths, preserving subdirectories

for file in *.html apps/*/*.html; do
  # CSS: /assets/css/foo.css or /assets/css/vendor/foo.css -> /assets/dist/css/foo.min.css or /assets/dist/css/vendor/foo.min.css
  sed -i '' -E 's#/assets/css/([a-zA-Z0-9/_-]+)\.css#/assets/dist/css/\1.min.css#g' "$file"
  # CSS (already minified): /assets/css/foo.min.css or /assets/css/vendor/foo.min.css -> /assets/dist/css/foo.min.css or /assets/dist/css/vendor/foo.min.css
  sed -i '' -E 's#/assets/css/([a-zA-Z0-9/_-]+)\.min\.css#/assets/dist/css/\1.min.css#g' "$file"
  # JS: /assets/js/foo.js or /assets/js/vendor/foo.js -> /assets/dist/js/foo.min.js or /assets/dist/js/vendor/foo.min.js
  sed -i '' -E 's#/assets/js/([a-zA-Z0-9/_-]+)\.js#/assets/dist/js/\1.min.js#g' "$file"
  # JS (already minified): /assets/js/foo.min.js or /assets/js/vendor/foo.min.js -> /assets/dist/js/foo.min.js or /assets/dist/js/vendor/foo.min.js
  sed -i '' -E 's#/assets/js/([a-zA-Z0-9/_-]+)\.min\.js#/assets/dist/js/\1.min.js#g' "$file"
done

echo "Switched HTML asset references to production /assets/dist/ paths, preserving subdirectories."