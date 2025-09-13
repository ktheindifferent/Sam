// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

const Clock = {
    interval: null,

    init: function() {
        this.start();
    },

    start: function() {
        this.displayTime();
        this.interval = setInterval(() => this.displayTime(), 1000);
    },

    stop: function() {
        if (this.interval) {
            clearInterval(this.interval);
            this.interval = null;
        }
    },

    displayTime: function() {
        const clockElement = document.getElementById('Clock');
        if (!clockElement) {
            console.warn('Clock element not found');
            return;
        }

        const timeNow = new Date();

        let hoursOfDay = timeNow.getHours();
        let minutes = timeNow.getMinutes();
        let seconds = timeNow.getSeconds();
        let weekDay = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
        let today = weekDay[timeNow.getDay()];
        let months = timeNow.toLocaleString("default", {
            month: "long"
        });
        let year = timeNow.getFullYear();
        let period = "AM";

        if (hoursOfDay > 12) {
            hoursOfDay -= 12;
            period = "PM";
        }

        if (hoursOfDay === 0) {
            hoursOfDay = 12;
            period = "AM";
        }

        hoursOfDay = hoursOfDay < 10 ? "0" + hoursOfDay : hoursOfDay;
        minutes = minutes < 10 ? "0" + minutes : minutes;
        seconds = seconds < 10 ? "0" + seconds : seconds;

        let time = hoursOfDay + ":" + minutes;

        clockElement.innerHTML = time;
    }
};

if (typeof window !== 'undefined') {
    window.Clock = Clock;
}