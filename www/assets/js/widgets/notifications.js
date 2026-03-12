

class Notifications {
    constructor(current_session) {
        this.current_session = current_session;
        this.notifications = [];
        this.toasted_notifications = [];
        this.is_open = false;
    }


    refreshUnseen(){
        var ref = this;
        $.get("/api/services/notifications/unseen")
            .done(function( data ) {
                var reversed = data.reverse();
                $(reversed).each(function(i, obj) {
                    console.log(obj.message);
                    if(!obj.seen && !ref.toasted_notifications.includes(obj.oid)){

                        ref.toasted_notifications.push(obj.oid);
                        setTimeout(() => { 
                            
                            toastr.info(obj.message, '', 
                                {onclick: function() {
                                    ref.markAsSeen(obj.oid);
                                }}
                            );
                        
                        }, 100);


                    
                    
                }
            });
        })
        .fail(function(xhr, status, error) {
            // Gracefully handle notifications service unavailable
            if (xhr.status === 404 || xhr.status === 302) {
                // Likely authentication issue - user may need to log in
                console.debug('Notifications service unavailable - authentication required:', status);
            } else {
                console.debug('Notifications service unavailable:', status, error);
            }
        });
    }

    refresh(){
        var ref = this;
        $.get("/api/services/notifications")
            .done(function( data ) {
                ref.notifications = data;
                if(ref.is_open){
                    $("#notifications_container").html(ref.genHtml());
                }
            })
            .fail(function(xhr, status, error) {
                if (xhr.status === 404 || xhr.status === 302) {
                    // Likely authentication issue - user may need to log in
                    console.debug('Notifications service unavailable - authentication required:', status);
                } else {
                    console.debug('Notifications service unavailable:', status, error);
                }
            });


    }

    markAsSeen(oid){
        $.post("/api/services/notifications/seen", { oid: oid } );
    }

    open() {
        this.is_open = true;
        // this.refresh();
        var ref = this;
        $("body").append(`<div id='notifications_container' class='notifications-container'>

            ${ref.genHtml()}

        </div>`);
    }

    _escapeHtml(str) {
        var div = document.createElement('div');
        div.appendChild(document.createTextNode(str || ''));
        return div.innerHTML;
    }

    genHtml(){
        var html = "";
        var ref = this;
        html += `<button onclick="notifications.close()" title="Close" class="btn btn-link notifications-exit-btn" ><i class="fas fa fa-times"></i></button>`;

        $(this.notifications).each(function(i, obj) {

            html += `<div class="notification-item">
                        <p>${ref._escapeHtml(obj.message)}</p>
                        <small>${ref._escapeHtml(obj.timestamp)}</small>
                    </div>`;
        });

        return html;

    }

    close(){
        this.is_open = false;
        $("#notifications_container").hide();
        $("#notifications_container").remove();
    }


    new(message){
        $.post("/api/services/notifications", { message: message } );
    }
}

