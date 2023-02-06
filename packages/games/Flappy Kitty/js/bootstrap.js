var vueSettingsMenu = null;

$(document).ready(function() {


  // Custom
  refreshUpdateHighScore();

});

wormbox.events.addEventListener('multiplayer_menu_opened', function(){
  setTimeout(function(){ 
    refreshUpdateHighScore();
  }, 500);
});

window.addEventListener("resize", function() {

  // $("#screen canvas").remove();
  rebuildGame();
});



function refreshUpdateHighScore(){
    // Refresh High score from server and update if local is better
    if(localStorage.flappy_high_score === undefined){
      localStorage.flappy_high_score = 0;
    }
    // Update Wormbox with Invasion Highscore
    setTimeout(function(){ 
      if(wormbox !== undefined){
        if(wormbox.user !== undefined && wormbox.user !== null){
          if(wormbox.user.user !== undefined && wormbox.user.user !== null){
            if(wormbox.user.user.games !== undefined && wormbox.user.user.games !== null){
              if(wormbox.user.user.games.flappykitty !== undefined && wormbox.user.user.games.flappykitty !== null){
                if(localStorage.flappy_high_score !== undefined){
                  if(!Number.isInteger(wormbox.user.user.games.flappykitty.highscore)){
                    wormbox.user.user.games.flappykitty.highscore = 0;
                  }
                  if(parseInt(localStorage.flappy_high_score) > parseInt(wormbox.user.user.games.flappykitty.highscore)){
                    wormbox.user.user.games.flappykitty.highscore = parseInt(localStorage.flappy_high_score);
                    wormbox.user.update(wormbox.user.user);
                  }
                  localStorage.flappy_high_score = parseInt(wormbox.user.user.games.flappykitty.highscore);
                }
              }
            }
          }
        }
      
      }

    }, 500);
}




// Cordova App Container
var app = {
    // Application Constructor
    initialize: function() {
        document.addEventListener('deviceready', this.onDeviceReady.bind(this), false);
    },

    onDeviceReady: function() {
        this.receivedEvent('deviceready');
        monitize();
        refreshUpdateHighScore();

      

        // loadSettingsMenu(true);
        if(cordova.platformId === "android"){
          cordova.plugins.firebase.dynamiclinks.onDynamicLink(function(data) {
              // data.deepLink
              var key = data.deepLink.split("/accept_friendship/")[1];

              // Listen for changes to users data
              wormbox.firebase.database().ref('friendships/' + key).once('value').then(function(snapshot) {
                if(snapshot.val() === null){
                  toastr.error(i18n("Friendship request expired...."));
                } else {
                  if(wormbox.user !== null && wormbox.user !== undefined && wormbox.user.user.uid === snapshot.val().user_0){
                    toastr.error(i18n("You can't accept your own friendship request...."));
                  } else {
                    if(snapshot.val().is_accepted === false){
                      wormbox.notification.showNewFriendRequest(snapshot);
                    } else {
                      toastr.error(i18n("Link expired...."));
                    }
                  }
                }
              });
        
          });
        }
            
        
    },

    // Update DOM on a Received Event
    receivedEvent: function(id) {
        // console.log('Received Event: ' + id);
    }
};

app.initialize();



wormbox.events.addEventListener('init_language', function(){
    refreshUpdateHighScore();

    var menuItems = {};


    menuItems["all_games"] = { title: i18n('Home'), isActive: false, isHidden: true, key: 'all_games'}

    if(window.location.href.includes("wormbox.online") || window.location.href.includes(":3000")){
      menuItems["all_games"].isHidden = false;
    }


    menuItems["play_game"] = { title: i18n('Play Game'), isActive: true, isHidden: true, key: 'play_game'}

    menuItems["leaderboard"] = { title: i18n('Leaderboard'), isActive: false, isHidden: !(navigator.onLine), key: 'leaderboard'}
    menuItems["pause_music"] = { title: i18n('Pause Music'), isActive: false, isHidden: false, key: 'pause_music'}
    menuItems["play_music"] = { title: i18n('Play Music'), isActive: false, isHidden: true, key: 'play_music'}
    menuItems["remove_ads"] = { title: i18n('Remove Ads ($0.99)'), isActive: false, isHidden: true, key: 'remove_ads' }


    //admob.createBannerView({publisherId: "ca-app-pub-9895414383509527/6485013533"});
  
    var ua = navigator.userAgent;
    var isKindle = /Kindle/i.test(ua) || /Silk/i.test(ua) || /KFTT/i.test(ua) || /KFOT/i.test(ua) || /KFJWA/i.test(ua) || /KFJWI/i.test(ua) || /KFSOWI/i.test(ua) || /KFTHWA/i.test(ua) || /KFTHWI/i.test(ua) || /KFAPWA/i.test(ua) || /KFAPWI/i.test(ua) || /KFFOWI/i.test(ua);

    if(cordova !== undefined){
      if(cordova.platformId !== undefined){
        if((localStorage.runAds === undefined || localStorage.runAds !== "false") && (cordova.platformId === "android" || cordova.platformId === "ios") && (!isKindle)){
          menuItems["remove_ads"].isHidden = false;
        } 
      }
    }

    menuItems["about"] = { title: i18n('About'), isActive: false, isHidden: false, key: 'about' }


    if(vueSettingsMenu === null){
      vueSettingsMenu = new Vue({
        el: '#settings',
        vuetify: new Vuetify({icons: {
          iconfont: 'mdi'
        }}),
        data: () => ({
          items: menuItems,
        }),
        methods: {
          changeGameMode(key) {
            console.log(key);
            switch(key) {
              case 'play_game':
                this.deactivateMenuItems();
                menuItems["play_game"].isActive = true;
                startGame();
                break;
              case 'leaderboard':
                if(!wormbox.auth.isUserAuthenticated()){
                  wormbox.auth.displaySignIn();
                } else {
                  refreshMultiPlayerData();
                  wormbox.menues.joinFriend.show();
                }
                break;
              case 'pause_music':
                menuItems["pause_music"].isHidden = true;
                menuItems["play_music"].isHidden = false;
                wormbox.music.pause();
                break;
              case 'play_music':
                menuItems["pause_music"].isHidden = false;
                menuItems["play_music"].isHidden = true;
                wormbox.music.play();
                break;
              case 'all_games':
                document.querySelector('#loading').style.display = 'block';
                document.querySelector('#loading_overlay').style.display = 'block';
                window.location.href = "/";
                break;
              case 'remove_ads':
                requestRemoveAds();
                break;
              case 'about':
                wormbox.notification.showMessage("Flappy Kitty V1.2.0<br/>Copyright (C) 2019 Caleb Smith Woolrich - Virginia Worm Company <br/><br/> \
                Based on HTML5 flappy bird (MIT License) \
                http://hyspace.io/flappy/ \
                <br/><br/>\
                Tic Tac Toe is made possible by the following open source projects and media.<br/>\
                Virginia Worm Company has NO affiliation with the open source project/media maintainers and creators.<br/><br/>\
                Permenent Marker Font by Principal design (Apache License, Version 2.0) <br/><br/> \
                Material Design Icon by Copyright (c) 2014, Austin Andrews Open Font License 1.1 (http://materialdesignicons.com/)<br/><br/>\
                Online/Offline Detection Provided by: \
                HubSpot/offline is licensed under the MIT License \
                Copyright (c) 2014 HubSpot, Inc. \
                https://github.com/HubSpot/offline/blob/master/LICENSE<br/><br/>\
                jquery/jquery is licensed under the MIT License \
                Copyright JS Foundation and other contributors, https://js.foundation/<br/><br/>\
                toastr \
                CodeSeven/toastr is licensed under the MIT License \
                Copyright (c) 2017 Toastr Maintainers \
                https://github.com/CodeSeven/toastr/blob/master/LICENSE<br/><br/>\
                vue \
                vuejs/vue is licensed under the MIT License \
                Copyright (c) 2013-present, Yuxi (Evan) You \
                https://github.com/vuejs/vue/blob/dev/LICENSE<br/><br/>\
                vuetify \
                vuetifyjs/vuetify is licensed under the MIT License \
                Copyright (c) 2016-2019 John Jeremy Leider \
                https://github.com/vuetifyjs/vuetify/blob/master/LICENSE.md<br/><br/>\
                html2canvas \
                html2canvas is Licensed under MIT license \
                Copyright (c) 2014 Yehuda Katz, Tom Dale, Stefan Penner and contributors (Conversion to ES6 API by Jake Archibald)<br/><br/>\
                i81n \
                nodejs/i18n is licensed under the MIT License \
                Copyright 2018<br/><br/>\
                lodash \
                lodash is licensed under the MIT License \
                Based on Underscore.js, copyright Jeremy Ashkenas, \
                DocumentCloud and Investigative Reporters & Editors \
                http://underscorejs.org/<br/><br/>\
                Font Family: Press Start 2P \
                Copyright (c) CodeMan38 Principal design \
                Open Font License \
                https://fonts.google.com/specimen/Press+Start+2P<br/><br/>\
                Music Provided By: \
                (C) patrickdearteaga \
                Creative Commons Attribution license (CC-BY) \
                https://patrickdearteaga.com/arcade-music/<br/><br/>\
                Boy free icon (C) Free Pik \
                Flaticon Basic License. \
                https://file000.flaticon.com/downloads/license/license.pdf<br/><br/>\
                ", "Credits");
                break;
              default:
                this.changeGameMode('play_game');
            }
          },
          deactivateMenuItems(){
            menuItems["play_game"].isActive = false;
            menuItems["leaderboard"].isActive = false;
            menuItems["pause_music"].isActive = false;
            menuItems["play_music"].isActive = false;
            menuItems["all_games"].isActive = false;
            menuItems["remove_ads"].isActive = false;
            menuItems["about"].isActive = false;
          },
          signOut(){
            wormbox.auth.signOut();
          },
          afterCare(){
            wormbox.events.dispatchEvent('language_changed');
          }
        }
      });

      wormbox.windowManager.init();
      wormbox.auth.init(false);

      menuItems["pause_music"].isHidden = true;
      menuItems["play_music"].isHidden = true;
    }
    wormbox.events.dispatchEvent('js');
    //wormbox.music.init();
    // if(localStorage.pauseMusic !== undefined){
    //   if(localStorage.pauseMusic === "yes"){
    //     menuItems["pause_music"].isHidden = true;
    //     menuItems["play_music"].isHidden = false;
    //   }
    // }
});

function requestRemoveAds(){
  if(cordova.platformId === "android" || cordova.platformId === "ios"){

    inAppPurchase.getProducts(['com.virginiawormco.flappykitty.remove_ads']).then(function (products) {
      console.log(products);
      inAppPurchase.buy('com.virginiawormco.flappykitty.remove_ads').then(function (data) {
        // Refresh window without AD's
        window.location.href = window.location.href;
      }).catch(function (err) {
        console.log(err);
      });
    }).catch(function (err) {
      console.log(err);
    });
  }
}

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