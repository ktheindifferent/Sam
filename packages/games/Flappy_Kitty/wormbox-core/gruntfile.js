module.exports = function(grunt) {

  // Project configuration.
  grunt.initConfig({
    pkg: grunt.file.readJSON('package.json'),
    terser: {
        options: {
            sourceMap: true
        },
        build: {
            files: {
                './public/wormbox.min.js': ['./vendor/firebase/firebase-app.js', './vendor/firebase/firebase-analytics.js', './vendor/firebase/firebase-auth.js', './vendor/firebase/firebase-database.js', './vendor/*.js', './tools/*.js', './modals/*.js', './ui/*.js', './ui/menus/*.js'],
            }
        }
    }
  });

  // Load the plugin that provides the "uglify" task.
  grunt.loadNpmTasks('grunt-terser');

  // Default task(s).
  grunt.registerTask('default', ['terser']);

};