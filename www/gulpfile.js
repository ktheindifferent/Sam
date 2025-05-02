const gulp = require('gulp');
const cleanCSS = require('gulp-clean-css');
const uglify = require('gulp-uglify');
const rename = require('gulp-rename');
const gulpIf = require('gulp-if');
const path = require('path');

function notMinified(file) {
  return !file.basename.endsWith('.min.js') && !file.basename.endsWith('.min.css');
}

// Minify JS (skip renaming if already .min.js)
gulp.task('minify-js', function () {
  return gulp.src(['assets/js/**/*.js'])
    .pipe(uglify())
    .pipe(gulpIf(
      file => !file.basename.endsWith('.min.js'),
      rename({ suffix: '.min' })
    ))
    .pipe(gulp.dest('assets/dist/js'));
});

// Minify CSS (skip renaming if already .min.css)
gulp.task('minify-css', function () {
  return gulp.src(['assets/css/**/*.css'])
    .pipe(cleanCSS())
    .pipe(gulpIf(
      file => !file.basename.endsWith('.min.css'),
      rename({ suffix: '.min' })
    ))
    .pipe(gulp.dest('assets/dist/css'));
});

gulp.task('default', gulp.parallel('minify-js', 'minify-css'));
