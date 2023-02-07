var vueSettingsMenu = null;

$(document).ready(function() {


  // Custom
  // refreshUpdateHighScore();

});

// wormbox.events.addEventListener('multiplayer_menu_opened', function(){
//   setTimeout(function(){ 
//     refreshUpdateHighScore();
//   }, 500);
// });

window.addEventListener("resize", function() {

  // $("#screen canvas").remove();
  rebuildGame();
});




  rebuildGame = function(){
    DEBUG = false;

    SPEED = 160;

    GRAVITY = 1100;

    FLAP = 320;

    SPAWN_RATE = 1 / 1200;

    OPENING = 400;

    SCALE = 1;

    HEIGHT = window.innerHeight;

    WIDTH = window.innerWidth;

    GAME_HEIGHT = window.innerHeight;

    GROUND_HEIGHT = 64;

    GROUND_Y = HEIGHT - GROUND_HEIGHT;

    floor = Math.floor;

    try{
     
      Phaser.Canvas.setSmoothingEnabled(game.context, false);
      game.stage.scale.height = window.innerHeight
      game.stage.scale.width = window.innerWidth
      game.stage.scaleMode = Phaser.StageScaleMode.AUTO;
      game.stage.scale.setScreenSize(true);
      game.world.width = WIDTH;
      game.world.height = HEIGHT;
      game.width = WIDTH;
      game.height = HEIGHT;
      bg.width = WIDTH;
      bg.height = HEIGHT;


      scoreText.x = game.world.width / 2
      scoreText.y = game.world.height / 4

      ground.x = 0
      ground.y = GROUND_Y
      ground.width = window.innerWidth;

      instText.x = game.world.width / 2
      instText.y = game.world.height - game.world.height / 4


      gameOverText.x = game.world.width / 2
      gameOverText.y = game.world.height / 2

      document.getElementsByTagName("canvas")[0].width = window.innerWidth;
      document.getElementsByTagName("canvas")[0].height = window.innerHeight;
      document.getElementsByTagName("canvas")[0].style.width = window.innerWidth + "px";
      document.getElementsByTagName("canvas")[0].style.height = window.innerHeight + "px";
      scoreText.setText(i18n("Flappy Kitty"));
      instText.setText(i18n("TOUCH TO FLAP"));
      gameOverText.renderable = false;
  
      bird.destroy()

      bird = game.add.sprite(0, 0, "bird");
      bird.anchor.setTo(0.5, 0.5);
      // bird.body.collideWorldBounds = true;
      // bird.body.setPolygon(24, 1, 34, 16, 30, 32, 20, 24, 12, 34, 2, 12, 14, 2);

      bird.body.allowGravity = false;
      bird.reset(game.world.width * 0.3, game.world.height / 2);
      bg.reset(0, 0)
      bird.angle = 0;
      bird.animations.play("fly");


      invs.removeAll();
      tubes.destroy();
      game.time.events.remove(tubesTimer);
      tubes = game.add.group();
      gameStarted = false;
    } catch(err){

    }

  }