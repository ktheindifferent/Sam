// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.


var audio_tell_me_about_you = new Audio('/assets/audio/setup/tell_me_about_you.wav');
var audio_tell_me_about_install_location = new Audio('/assets/audio/setup/tell_me_about_install_location.wav');
var audio_connect_srv_things = new Audio('/assets/audio/setup/connect_srv_things.wav');





document.addEventListener('DOMContentLoaded', function() {

  document.getElementById("clickme").addEventListener("click", function(evnt) {
    alert("Click anywhere to start the video.");
    var intro_video = document.getElementById('intro_video');
    intro_video.play().catch(function(){});
  });

  var played_intro_video = false;
  var intro_video_started = false;

  var canvas = document.getElementById('intro_video_canvas');
  var ctx = canvas.getContext('2d');
  var intro_video = document.getElementById('intro_video');

  // Mute the video and audio to allow autoplay
  intro_video.muted = true;
  audio_tell_me_about_you.muted = true;
  audio_tell_me_about_install_location.muted = true;
  audio_connect_srv_things.muted = true;

  // set canvas size = video size when known
  intro_video.addEventListener('loadedmetadata', function() {
    canvas.width = intro_video.videoWidth;
    canvas.height = intro_video.videoHeight;
  });

  intro_video.addEventListener('play', function() {
    document.getElementById("clickme").style.display = "none";
    document.getElementById("intro_video_canvas").style.display = "";
    var $this = this; //cache
    (function loop() {
    if (!$this.paused && !$this.ended) {
      ctx.drawImage($this, 0, 0, canvas.width, canvas.height);
      setTimeout(loop, 1000 / 30); // drawing at 30fps
    }
    })();
  });

  intro_video.addEventListener('ended', function() {
    played_intro_video = true;
    document.getElementById("intro_video_canvas").style.display = "none";
    document.getElementById("intro_video").style.display = "none";
    document.getElementById("setup_step_1_card").style.display = "";
    audio_tell_me_about_you.muted = false;
    audio_tell_me_about_you.play().catch(function(){});
  });

  document.body.addEventListener('click', function bodyClick(evnt) {
    if(!intro_video_started){
    intro_video_started = true;
    // Unmute after user interaction
    intro_video.muted = false;
    audio_tell_me_about_you.muted = false;
    audio_tell_me_about_install_location.muted = false;
    audio_connect_srv_things.muted = false;
    intro_video.play().catch(function(){});
    document.body.removeEventListener('click', bodyClick);
    }
  });

  document.getElementById('step_1_nxt_btn').addEventListener('click', function step1Next(evnt) {
    document.getElementById("setup_step_1_card").style.display = "none";
    audio_tell_me_about_install_location.muted = false;
    audio_tell_me_about_install_location.play().catch(function(){});
    document.getElementById("setup_step_2_card").style.display = "";
    this.removeEventListener('click', step1Next);
  });

  document.getElementById("step_2_prv_btn").addEventListener('click', function(evnt) {
    document.getElementById("setup_step_2_card").style.display = "none";
    document.getElementById("setup_step_1_card").style.display = "";
  });

  document.getElementById('step_2_nxt_btn').addEventListener('click', function step2Next(evnt) {
    document.getElementById("setup_step_2_card").style.display = "none";
    audio_connect_srv_things.muted = false;
    audio_connect_srv_things.play().catch(function(){});
    document.getElementById("setup_step_3_card").style.display = "";
    this.removeEventListener('click', step2Next);
  });

  document.getElementById("step_3_prv_btn").addEventListener('click', function(evnt) {
    document.getElementById("setup_step_3_card").style.display = "none";
    document.getElementById("setup_step_2_card").style.display = "";
  });

});