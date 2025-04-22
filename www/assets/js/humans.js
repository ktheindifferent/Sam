// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

var humans_table = $("#humans_table");
var humans_table_body = $("#humans_table_body");

const queryString = window.location.search;
const urlParams = new URLSearchParams(queryString);
const oid = urlParams.get('oid');

$(document).ready(function() {

    if(oid !== null && oid !== undefined) {
        initialzeHumanProfile(oid);
    } else {
        initialzeHumansTable();
    }
    
    
});

function initialzeHumanProfile(oid){
    $("#human_profile_group").show();


    $.get(`/api/humans/${oid}`, function( data ) {
        $("#human_name").val(data.name);
        $("#human_email").val(data.email);
        $("#human_phone_number").val(data.phone_number);
    });

    
}

function initialzeHumansTable() {
    $("#humans_index_group").show();
    var html = "";
    $.get("/api/humans", function( data ) {
        $(data).each(function() {
            html += `
                <tr>
                    <td style="font-size: 12px;">
                        <a href="/humans.html?oid=${this.oid}">
                            ${this.name}
                            <br/>
                            ${this.email}
                        </a>
                    </td>
                </td>
            `;
        });
        $("#humans_table_body").html(html);
    });
}