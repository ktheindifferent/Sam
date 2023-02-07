var testMode = false;

var advertisingIds = {
	admob: {
		banner: 'ca-app-pub-9895414383509527/4875299802', // or DFP format "/6253334/dfp_example_ad"
		interstitial: 'ca-app-pub-9895414383509527/7285840728',
    enable_interstitial: false
	},
	facebook: {
		banner: 'ca-app-pub-9895414383509527/6485013533', // or DFP format "/6253334/dfp_example_ad"
		interstitial: 'ca-app-pub-9895414383509527/3475955201',
    enable_interstitial: false
	},
  amazon_mobile_ad: {
    key: ' 087c1139002a4633885419eb5708da86',
    enable_interstitial: false
  },
	admob_dev: {
		banner: 'ca-app-pub-3940256099942544/6300978111', // or DFP format "/6253334/dfp_example_ad"
		interstitial: 'ca-app-pub-3940256099942544/1033173712',
    enable_interstitial: false
	},
	facebook_dev: {
		banner: 'ca-app-pub-3940256099942544/6300978111', // or DFP format "/6253334/dfp_example_ad"
		interstitial: 'ca-app-pub-3940256099942544/1033173712',
    enable_interstitial: false
	}
};

// Test Mode Overide
// -----------------------------------------------------
if(testMode){
  console.log("Monitize entering test mode....");
  advertisingIds.admob.banner = advertisingIds.admob_dev.banner;
  advertisingIds.admob.interstitial = advertisingIds.admob_dev.interstitial;
}


function monitize(){

  

	var device = {
		type: "Unknown"
	}

  var ua = navigator.userAgent;
  var isElectron = /electron/i.test(ua);
  if(isElectron) { 
    device.type = "electron";
  }

  var ua = navigator.userAgent.toLowerCase();
  var isAndroid = /android/i.test(ua);
  if(isAndroid) {
    device.type = "android";
  }

	var ua = navigator.userAgent;
	var isKindle = /Kindle/i.test(ua) || /Silk/i.test(ua) || /KFTT/i.test(ua) || /KFOT/i.test(ua) || /KFJWA/i.test(ua) || /KFJWI/i.test(ua) || /KFSOWI/i.test(ua) || /KFTHWA/i.test(ua) || /KFTHWI/i.test(ua) || /KFAPWA/i.test(ua) || /KFAPWI/i.test(ua) || /KFFOWI/i.test(ua);
	if(isKindle) { 
		device.type = "kindle";
	}

  if(localStorage.runAds !== undefined){
    if(localStorage.runAds === "false"){
      return;
    }
  }

	if(device.type === "android"){
    if(cordova.platformId === "android"){
      inAppPurchase
          .getProducts(["com.virginiawormco.flappykitty.remove_ads"])
          .then(function (products) {
            console.log(products);
            inAppPurchase
            .restorePurchases()
            .then(function (data) {
              console.log(data);

              if(data.length === 0){
                localStorage.runAds = true;
                startAds("admob");
              } else {
                if(data[0].receipt.length === 0){
                  localStorage.runAds = true;
                  startAds("admob");
                } else {
                  localStorage.runAds = false;
                }
              }

            })
            .catch(function (err) {
              startAds("admob");
              console.log(err);
            });
          }).catch(function (err) {
            startAds("admob");
            console.log(err);
          });
    }
		console.log("Monitize Android Device");
	}

	if(device.type === "kindle"){
		console.log("Monitize Kindle Device");
    startAds("amazon_ads");
	}

  if(device.type === "electron"){
    
    console.log("Monitize Electron Device");
  }

  // Show Donate Button for Web and Electron (Web, Windows, Linux, Mac)
  if(cordova.platformId !== "android" && cordova.platformId !== "ios"){
    // var x = document.getElementById("donate_button");
    // x.style.display = "block";    
    // var x = document.getElementById("desktop_btns");
    // x.style.display = "block"; 
  }



}

function startAds(platform){

  // Android - Use Ad Mob
  if(platform === "admob"){
    console.log("Monitize with AdMob");
    
    if(localStorage.runAds === undefined || localStorage.runAds !== "false"){
      advertisingIds.admob.enable_interstitial = true;
      // if(AdMob) AdMob.createBanner({
      //   adId: advertisingIds.admob.banner,
      //   autoShow: true, // TODO: remove this line when release
      //   overlap: true,
      //   offsetTopBar: false,
      //   bgColor: 'black' }, succeededToLoadAdMob, failedToLoadAdMob);
    }
  }

  if(platform === "amazon_ads"){
    console.log("Monitize with Amazon Ads");
    window.amazonads = new AmazonMobileAds();
    // get things started by passing in your app key
    amazonads.init(advertisingIds.amazon_mobile_ad.key, function() {

      // For testing Amazon Ads
      // -------------------------------------------------
      if(testMode){
        amazonads.enableTestMode(function() {
          console.log('testing 1 2 1 2');
        }, function(err) {
          console.error('this will never ever happen');
        });
      }

      console.log('super dope it worked');
      advertisingIds.amazon_mobile_ad.enable_interstitial = true;
        amazonads.showBannerAd(false, true, function() {
          console.log('what what see the banner at the bottom');
        }, function(err) {
          console.error(['oh crap', err]);
          startAds("admob");
        });
    }, function(err) {
      console.error(['oh crap', err]);
      startAds("admob");
    });
  }

}
function succeededToLoadAdMob(){
  console.log("Ad mob win!");
  // AdMob.showBanner();
}
function failedToLoadAdMob(){
  console.log("Ad mob failed");
}





// Amazon Web Platform
// =========================================
// var _AmazonPlatformUtils=new function(){var a=/AmazonWebAppPlatform\/([\d\.]+)?;([\d\.]+)?/g,b="";"undefined"!==typeof window&&(b=window.navigator.userAgent,a=a.exec(b),null!=a&&(this.platformFullVersion=a[1],this.bridgeInterfaceVersion=a[2]));this.platformVersion=this.platformFullVersion?this.platformFullVersion.split(".")[0]:-1;this.platformEnabled=0<this.platformVersion;this.isPlatformEnabled=function(){return this.platformEnabled};this.getPlatformFullVersion=function(){return this.platformFullVersion};
// this.getBridgeInterfaceVersion=function(){return this.bridgeInterfaceVersion};this.isCordovaEnabled=function(){return this.platformEnabled&&3<=this.platformVersion?!0:!1};this.cordovaReady=!1;this.cordovaCommandQueue=[];this.runWhenCordovaReady=function(c){this.cordovaReady?c():this.cordovaCommandQueue.push(c)};this.setCordovaReady=function(){this.cordovaReady=!0};this.injectScript=function(c,a,d){var b=document.createElement("script");a||(a=function(){"undefined"!=typeof console&&console.log&&console.log("Loaded resource : "+
// c)});d||(d=function(){"undefined"!=typeof console&&console.log&&console.log("Failed loading resource : "+c+". App may not function properly")});b.onload=a;b.onerror=onerror;b.src=c;document.head.appendChild(b)}};_AmazonPlatformUtils.isCordovaEnabled()&&document.addEventListener("deviceready",function(){_AmazonPlatformUtils.setCordovaReady();for(var a=0;a<_AmazonPlatformUtils.cordovaCommandQueue.length;a++)_AmazonPlatformUtils.cordovaCommandQueue[a]()},!1);function _AmazonEnum(a){this._init(a)}
// _AmazonEnum.prototype={values:null,_init:function(a){for(var b=0;b<a.length;b++)this[a[b]]=a[b];this.values=a}};
// (function(){_AmazonServices=function(){this._sdkPlatformMode=_AmazonPlatformUtils.isPlatformEnabled()?this.SdkPlatformMode.ANDROID:this.SdkPlatformMode.JAVASCRIPT};_AmazonServices.prototype={SdkPlatformMode:new _AmazonEnum(["ANDROID","JAVASCRIPT","JAVASCRIPT_LIMITED"]),_sdkPlatformMode:null,get isAppstoreApp(){return this.sdkPlatformMode==this.SdkPlatformMode.ANDROID},get sdkPlatformMode(){return this._sdkPlatformMode}};if("undefined"==typeof amzn_wa||!amzn_wa)window.amzn_wa=new _AmazonServices})();
// (function(){var a=function(c){this.event=c};a.prototype={event:null,isFired:!1,fireData:null,listeners:[],fireEvent:function(c){this.isFired=!0;this.fireData=c;for(c=0;c<this.listeners.length;c++){var a=this.listeners[c];this.triggerHandler(a.handler,a.capture)}},triggerHandler:function(a){a.call(null,this.fireData)},addListener:function(a,b){this.isFired&&this.triggerHandler(a,b);this.listeners.push({handler:a,capture:b})},removeListener:function(a){for(var b=0;b<this.listeners.length;b++)if(this.listeners[b].handler===
// a){this.listeners.splice(b,1);break}}};window.DocumentEventHandler=function(){this.initialize()};var b=null;DocumentEventHandler.getDocumentEventHandler=function(){null==b&&(b=new DocumentEventHandler);return b};DocumentEventHandler.prototype={documentEventHandlers:{},initialize:function(){var a=this,b=document.addEventListener,d=document.removeEventListener;document.addEventListener=function(d,g,e){var h=a.documentEventHandlers,i=d.toLowerCase();h[i]?h[i].addListener(g,e):b.call(document,d,g,e)};
// document.removeEventListener=function(b,j,e){var h=a.documentEventHandlers,i=b.toLowerCase();h[i]?h[i].removeListener(j):d.call(document,b,j,e)}},fireDocumentEvent:function(a,b){var d=document.createEvent("Events");d.initEvent(a,!1,!1);if(b)for(var f in b)b.hasOwnProperty(f)&&(d[f]=data[f]);var g=this.documentEventHandlers,e=a.toLowerCase();g[e]?setTimeout(function(){document.dispatchEvent(d);g[e].fireEvent(d)},0):document.dispatchEvent(d)},addStickyDocumentEvent:function(b){this.documentEventHandlers[b.toLowerCase()]=
// new a(b)},API_READY_EVENT_NAME:"amazonPlatformReady"}})();var _AmazonLibraryLoaders=null;
// (function(){var a=null,b=function(){null==a&&(a=DocumentEventHandler.getDocumentEventHandler(),a.addStickyDocumentEvent(a.API_READY_EVENT_NAME));setTimeout(function(){a.fireDocumentEvent(a.API_READY_EVENT_NAME)},0)};_AmazonLibraryLoaders={wrapperLoader:new function(){this.loadLibrary=function(){document.addEventListener("deviceready",function(){b()},!1);_AmazonPlatformUtils.injectScript(this.getResourceUrl())};this.isApplicable=function(){_AmazonPlatformUtils.isPlatformEnabled();var a=_AmazonPlatformUtils.isCordovaEnabled(),
// b=_AmazonPlatformUtils.getPlatformFullVersion();return a&&"3"!=b&&0!=_AmazonPlatformUtils.getBridgeInterfaceVersion().indexOf("1.0")?!0:!1};this.getResourceUrl=function(){return"amzn-wa://webasset/cordova.js"}},legacyLoader:new function(){this.loadLibrary=function(){var a=_AmazonPlatformUtils.isCordovaEnabled();_AmazonPlatformUtils.injectScript("https://amazon-web-app-resources.s3.amazonaws.com/v0/latest/Amazon-Web-App-API.min.js",function(){a?document.addEventListener("deviceready",function(){b()},
// !1):b();console.log("Loaded Legacy resource : https://amazon-web-app-resources.s3.amazonaws.com/v0/latest/Amazon-Web-App-API.min.js")})};this.isApplicable=function(){var a=_AmazonPlatformUtils.isPlatformEnabled(),b=_AmazonPlatformUtils.isCordovaEnabled(),d=_AmazonPlatformUtils.getPlatformFullVersion();return b&&"3"==d||b&&0==_AmazonPlatformUtils.getBridgeInterfaceVersion().indexOf("1.0")||a&&!b?!0:!1}},openWebLoader:new function(){this.loadLibrary=function(){};this.isApplicable=function(){return!_AmazonPlatformUtils.isPlatformEnabled()}}}})();
// amzn_wa=function(a){function b(a){this.name=j;this.message=a}var c=[],j="WebAppBridgeError";a.addErrorCallback=function(a){c.push(a)};b.prototype=Error();a.throwException=function(a){try{a=JSON.parse(a),a.hasOwnProperty("message")&&(a=a.message)}catch(f){console.log(f)}c.forEach(function(b){b(a)});throw(new b(a)).toString();};return a}(amzn_wa);
// (function(){for(sLoader in _AmazonLibraryLoaders){var a=_AmazonLibraryLoaders[sLoader];if(a.isApplicable()){"undefined"!=typeof console&&console.log("Applying loader : "+sLoader+" to load libraries");a.loadLibrary();break}}})();



          