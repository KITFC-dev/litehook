async function fetchListeners() {
    try {
        const res = await fetch('http://127.0.0.1:4101/listeners');
        const data = await res.json();
        const container = document.getElementById('listeners-list');
        container.innerHTML = '';

        // List listeners
        data.forEach(listener => {
            const card = document.createElement('div');
            card.className = 'listener-card';
            
            card.innerHTML = `
                <div class="card-header">
                    <h3 class="listener-id">${listener.id}</h3>
                    <div class="listener-tags">
                        <span class="listener-tag">${listener.active ? 'Active' : 'Inactive'}</span>
                        <span class="listener-tag">Interval: ${listener.poll_interval} s</span>
                    </div>
                </div>
                <hr>
                <div class="card-body">
                    <p class="listener-attribute">Channel URL: <a href="${listener.channel_url}">${listener.channel_url}</a></p>
                    <p class="listener-attribute">Webhook URL: <a href="${listener.webhook_url}">${listener.webhook_url}</a></p>
                </div>
                <div class="listener-footer">
                    <h4>Controls</h4>
                    <button class="listener-control stop" data-action="stop" data-id="${listener.id}">Stop</button>
                    <button class="listener-control edit" data-action="edit" data-id="${listener.id}">Edit</button>
                </div>
            `;

            container.appendChild(card);
        });
    } catch (err) {
        document.getElementById('listeners-list').textContent = 'Error fetching listeners';
        console.error(err);
    }
}

fetchListeners();
setInterval(fetchListeners, 5000);
