
function escapeHtmlSearch(str) {
    var div = document.createElement('div');
    div.appendChild(document.createTextNode(str || ''));
    return div.innerHTML;
}

class SearchWidget {
    constructor() {
        this.is_visible = false;
        this.initialized = false;
        this.result_limit = 8;

        $("body").append(`<div id='search_widget_container' class='search-widget-container'>
        
        <input type="text" id="search_input" class="search-input" placeholder="Search for anything..." />
        <button onclick="search_widget.hide();" title="Close" class="btn btn-link search-exit-btn" ><i class="fas fa fa-times"></i></button>


            <div class='search-results' id='search_results'></div>

            <div class="bg-animation">
                <div id="stars"></div>
                <div id="stars2"></div>
                <div id="stars3"></div>
                <div id="stars4"></div>
            </div>

          
        </div>`);

        var ref = this;

        $("#search_input").keyup(function() {
            ref.reloadResults();
          });
        
        
    
    }

    reloadResults(){
        var ref = this;
        var html = "";

        let search_input = $("#search_input");

        $("#search_results").html(" ");


        // /api/services/media/games/games

        $.get(`/api/services/media/games/games`, function( data ) {
            $(data).each(function(i, obj) {


                var safeName = escapeHtmlSearch(obj.name);
                var safeIcon = escapeHtmlSearch(obj.icon);
                var gid = `game${safeName.replace(/\s/g, "")}`;


                html += `<div class='game-result' id="${gid}">

                    <img src='${safeIcon}' class='image'></img>
                    <p>${safeName}</p>
                    <div class="middle">
                        <button onclick="" class='btn btn-primary'><i class="fas fa-play"></i></button>
                    </div>

                </div>`;
                
                if($(`#${gid}`).length === 0){
                    $("#search_results").append(html);
                }
                
                
            });
        });

    
        $.get(`/api/services/media/youtube?q=${encodeURIComponent(search_input.val())}`, function( data ) {
            $(data).each(function(i, obj) {
                if(i >= ref.result_limit) {

                } else {
                    var video = obj["Video"];
                    if(video !== undefined) {
                        var safeTitle = escapeHtmlSearch(video.title);
                        var safeId = escapeHtmlSearch(video.id);
                        var safeThumb = (video.thumbnails && video.thumbnails[3]) ? escapeHtmlSearch(video.thumbnails[3].url) : '';
                        html += `<div class='video-result'>

                            <img src='${safeThumb}' class='image'></img>
                            <p>${safeTitle}</p>
                            <div class="middle">
                                <button onclick="new VideoPlayer('youtube:${safeId}');" class='btn btn-primary'><i class="fas fa-play"></i></button>
                                <button onclick="downloadYoutubeVideo('${safeId}');" class='btn btn-primary'><i class="fas fa-download"></i></button>
                            </div>

                        </div>`;
                    }

                    $("#search_results").append(html);
                }
            });
        });
    }

    show(){
        this.is_visible = true;
        $("#search_widget_container").show();
    }

    hide(){
        this.is_visible = false;
        $("#search_widget_container").hide();
    }
}

var search_widget = new SearchWidget();

function downloadYoutubeVideo(id){
    var safeId = encodeURIComponent(id);
    notifications.new(`Downloading video: ${escapeHtmlSearch(id)} from YouTube...`);
    $.get(`/api/services/media/youtube/download?id=${safeId}`, function( data ) {

    });
}