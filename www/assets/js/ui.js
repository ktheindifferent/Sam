document.addEventListener('DOMContentLoaded', function() {
    // Highlight "Humans" if URL contains "humans.html"
    var sidebarHtml = `
        <div class="sidebar-wrapper">
            <ul class="nav">
                <li class="${window.location.href.includes('index.html') ? 'active' : ''}">
                    <a href="./index.html">
                        <i class="fa fa-home"></i>
                        <p>Dashboard</p>
                    </a>
                </li>
                <li class="${window.location.href.includes('humans.html') ? 'active' : ''}">
                    <a href="./humans.html">
                        <i class="fa fa-users"></i>
                        <p>Humans</p>
                    </a>
                </li>
                <li class="${window.location.href.includes('locations.html') ? 'active' : ''}">
                    <a href="./locations.html">
                        <i class="fa fa-map"></i>
                        <p>Locations</p>
                    </a>
                </li>
                <li class="${window.location.href.includes('media.html') ? 'active' : ''}">
                    <a href="./media.html">
                        <i class="fa fa-play"></i>
                        <p>Media</p>
                    </a>
                </li>
                <li style="display: none;">
                    <a href="./pets.html">
                        <i class="fa fa-dog"></i>
                        <p>Pets</p>
                    </a>
                </li>
                <li class="${window.location.href.includes('things.html') ? 'active' : ''}">
                    <a href="./things.html">
                        <i class="fa fa-lightbulb"></i>
                        <p>Things</p>
                    </a>
                </li>
                <li class="${window.location.href.includes('services.html') ? 'active' : ''}">
                    <a href="./services.html">
                        <i class="fa fa-bars"></i>
                        <p>Services</p>
                    </a>
                </li>
                <li class="${window.location.href.includes('settings.html') ? 'active' : ''}">
                    <a href="./settings.html">
                        <i class="fa fa-cogs"></i>
                        <p>Settings</p>
                    </a>
                </li>
            </ul>
        </div>
    `;
    document.getElementById('sidebar').innerHTML = sidebarHtml;

    const isThingsPage = window.location.href.includes('things.html');
    document.getElementById('navbar').innerHTML = `
        <div class="container-fluid">
          <div class="navbar-wrapper">
            <div class="navbar-toggle d-inline">
              <button type="button" class="navbar-toggler">
                <span class="navbar-toggler-bar bar1"></span>
                <span class="navbar-toggler-bar bar2"></span>
                <span class="navbar-toggler-bar bar3"></span>
              </button>
            </div>
            <div id="Clock" class="glow">00:00:00</div>
            <!-- <a class="navbar-brand" href="#">Hello, <span class="inject-human-name">...</span></a> -->
          </div>
    
          <div class="search-bar input-group topbar-right-buttons">
            <button title="Observation Deck" class="btn btn-link" onclick="observation_deck.open();"><i class="fas fa-binoculars"></i></button>
            ${isThingsPage ? `
            <div class="dropdown">
              <button class="btn btn-link" type="button" id="dropdownMenuButton" data-toggle="dropdown" aria-haspopup="true" aria-expanded="false">
                <i class="fas fa-plus"></i>
              </button>
              <div class="dropdown-menu" aria-labelledby="dropdownMenuButton">
                <a class="dropdown-item" href="#" onclick="newThing('rtsp');">Camera</a>
                <a class="dropdown-item" href="#" onclick="newThing('matter');">Matter</a>
              </div>
            </div>
            ` : ''}
            <button title="Notifications" class="btn btn-link" style="margin-left: 0;" onclick="notifications.open();"><i class="fas fa-bell"></i></button>
            <button title="Software Store" class="btn btn-link" style="margin-left: 0;" onclick="oss.show();"><i class="fas fa-store-alt"></i></button>
            <button title="Search" class="btn btn-link" style="margin-left: 0;" onclick="search_widget.show();"><i class="fas fa-search"></i></button>
            <button title="Console" class="btn btn-link" style="margin-left: 0;" onclick="window.open('/apps/console/index.html', 'signup', 'menubar=no, location=no, toolbar=no, scrollbars=no, height=300'); return false;"><i class="fas fa-terminal"></i></button>
          </div>
        </div>
    `;
});
