// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.


// Storage provider management
let currentStorageProvider = 'sam';

$(document).ready(function() {

    // Load general settings
    $.get("/api/settings", function( data ) {
        console.log(data);

        var html = "";
        $(data).each(function() {
            html += `
            <tr>
                <td>${this.key}</td>
                <td><input type="text" value=${this.values} /></td>
            </tr>
            `;
        });
        $("#settings_table").html(html);
    });

    // Load storage settings
    loadStorageSettings();

    // Handle storage provider selection change
    $('#defaultStorageProvider').on('change', function() {
        const selectedProvider = $(this).val();
        showStorageConfig(selectedProvider);
    });

    // Show default provider config
    showStorageConfig('sam');
});

function showStorageConfig(provider) {
    // Hide all config panels
    $('.storage-config').hide();

    // Show selected provider config
    $(`#${provider}-config`).show();

    currentStorageProvider = provider;
    $('#defaultStorageProvider').val(provider);
}

function loadStorageSettings() {
    $.get("/api/storage/settings", function(data) {
        if (data.defaultProvider) {
            currentStorageProvider = data.defaultProvider;
            showStorageConfig(data.defaultProvider);
        }

        // Load provider configurations
        if (data.providers) {
            // NextCloud settings
            if (data.providers.nextcloud) {
                $('#nextcloudServerUrl').val(data.providers.nextcloud.serverUrl || '');
                $('#nextcloudUsername').val(data.providers.nextcloud.username || '');
                // Don't populate password for security
            }

            // Dropbox settings
            if (data.providers.dropbox) {
                $('#dropboxAccessToken').val(data.providers.dropbox.accessToken || '');
            }

            // SeaweedFS settings
            if (data.providers.seaweedfs) {
                $('#seaweedMasterUrl').val(data.providers.seaweedfs.masterUrl || 'http://localhost:9333');
                $('#seaweedFilerUrl').val(data.providers.seaweedfs.filerUrl || 'http://localhost:8888');
                $('#seaweedCollection').val(data.providers.seaweedfs.collection || 'sam');
                $('#seaweedReplication').val(data.providers.seaweedfs.replication || '000');
            }

            // SAM settings
            if (data.providers.sam) {
                $('#samStoragePath').val(data.providers.sam.basePath || '/var/sam/storage');
                $('#samEnableEncryption').prop('checked', data.providers.sam.enableEncryption || false);
                $('#samEnableCompression').prop('checked', data.providers.sam.enableCompression || true);
            }
        }
    }).fail(function() {
        console.log("Could not load storage settings - using defaults");
    });
}

function testStorageConnection(provider) {
    const config = gatherStorageConfig(provider);

    if (!config) {
        toastr.error('Please fill in all required configuration fields');
        return;
    }

    // Show loading state
    const button = event.target;
    const originalText = button.innerHTML;
    button.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Testing...';
    button.disabled = true;

    $.ajax({
        url: `/api/storage/test/${provider}`,
        type: 'POST',
        contentType: 'application/json',
        data: JSON.stringify(config),
        success: function(response) {
            if (response.success) {
                toastr.success(`${provider} connection successful!`);
            } else {
                toastr.error(`${provider} connection failed: ${response.message}`);
            }
        },
        error: function(xhr, status, error) {
            toastr.error(`${provider} connection test failed: ${error}`);
        },
        complete: function() {
            button.innerHTML = originalText;
            button.disabled = false;
        }
    });
}

function gatherStorageConfig(provider) {
    switch (provider) {
        case 'nextcloud':
            const serverUrl = $('#nextcloudServerUrl').val();
            const username = $('#nextcloudUsername').val();
            const password = $('#nextcloudPassword').val();

            if (!serverUrl || !username || !password) {
                return null;
            }

            return {
                serverUrl: serverUrl,
                username: username,
                password: password
            };

        case 'dropbox':
            const accessToken = $('#dropboxAccessToken').val();

            if (!accessToken) {
                return null;
            }

            return {
                accessToken: accessToken
            };

        case 'seaweedfs':
            const masterUrl = $('#seaweedMasterUrl').val();
            const filerUrl = $('#seaweedFilerUrl').val();

            if (!masterUrl || !filerUrl) {
                return null;
            }

            return {
                masterUrl: masterUrl,
                filerUrl: filerUrl,
                collection: $('#seaweedCollection').val() || 'sam',
                replication: $('#seaweedReplication').val() || '000'
            };

        case 'sam':
            return {
                basePath: $('#samStoragePath').val() || '/var/sam/storage',
                enableEncryption: $('#samEnableEncryption').is(':checked'),
                enableCompression: $('#samEnableCompression').is(':checked')
            };

        default:
            return null;
    }
}

function saveStorageSettings() {
    const config = gatherStorageConfig(currentStorageProvider);

    if (!config) {
        toastr.error('Please fill in all required configuration fields');
        return;
    }

    const settings = {
        defaultProvider: currentStorageProvider,
        providers: {}
    };

    settings.providers[currentStorageProvider] = config;

    $.ajax({
        url: '/api/storage/settings',
        type: 'POST',
        contentType: 'application/json',
        data: JSON.stringify(settings),
        success: function(response) {
            if (response.success) {
                toastr.success('Storage settings saved successfully!');
            } else {
                toastr.error('Failed to save storage settings: ' + response.message);
            }
        },
        error: function(xhr, status, error) {
            toastr.error('Failed to save storage settings: ' + error);
        }
    });
}

function authenticateDropbox() {
    // Open Dropbox OAuth flow
    $.get('/api/storage/dropbox/auth', function(response) {
        if (response.authUrl) {
            // Open auth URL in new window
            const authWindow = window.open(response.authUrl, 'dropbox-auth', 'width=600,height=400');

            // Poll for completion
            const checkAuth = setInterval(function() {
                if (authWindow.closed) {
                    clearInterval(checkAuth);
                    // Check if authentication was successful
                    checkDropboxAuth();
                }
            }, 1000);
        }
    }).fail(function() {
        toastr.error('Failed to initiate Dropbox authentication');
    });
}

function checkDropboxAuth() {
    $.get('/api/storage/dropbox/status', function(response) {
        if (response.authenticated) {
            $('#dropboxAccessToken').val(response.accessToken);
            toastr.success('Dropbox authentication successful!');
        }
    });
}
