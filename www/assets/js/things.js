// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2023 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

let config = {
    baseUrl: "ws://127.0.0.1:1780"
};

var things = [];
$(document).ready(function() {

   
    $.get("/api/things", function( data ) {



        $(data).each(function() {
            things.push(this);

            if(this.thing_type == "lifx") {
                // init_lifx_thing_group(this);
                // init_lifx_thing(this);
                var x = new LifXThing(this.oid);
                x.init();

                // TODO - Fix Groups
            }

        });
       


   
    });
    
    
});

function newThing(type){
    if(type == 'rtsp'){
        Swal.fire({
            title: 'New Camera (RTSP)',
            showCancelButton: true,
            showConfirmButton: false,
            html: `
            
                <form action="/api/things" method="post" >

                    <input type="hidden" name="new_thing_type" id="new_thing_type" value="rtsp" />

                    <div class="form-group">
                        <label for="new_thing_name">Name</label>
                        <input type="text" class="form-control" id="new_thing_name" name="new_thing_name"/>
                    </div>

                    <div class="form-group">
                        <label for="new_thing_ip">IP Address</label>
                        <input type="text" class="form-control" id="new_thing_ip" name="new_thing_ip"/>
                    </div>

                    <div class="form-group">
                        <label for="new_thing_username">Username</label>
                        <input type="text" class="form-control" id="new_thing_username" name="new_thing_username"/>
                    </div>

                    <div class="form-group">
                        <label for="new_thing_password">Password</label>
                        <input type="password" class="form-control" id="new_thing_password" name="new_thing_password"/>
                    </div>

                    <button type="submit" class="btn btn-primary">Save</button>

                </form>
            
            `
          });


          
    }
}