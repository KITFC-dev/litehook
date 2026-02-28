const style = getComputedStyle(document.documentElement);
const bgColor = style.getPropertyValue('--bg-color').trim();
const mantle = style.getPropertyValue('--mantle').trim();
const textColor = style.getPropertyValue('--text-color').trim();
const accentColor = style.getPropertyValue('--accent-color').trim();

async function fetchListeners() {
    try {
        const res = await fetch('/listeners');
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
                    <button class="listener-control error" data-action="delete" data-id="${listener.id}">Delete</button>
                    <button class="listener-control info" data-action="edit" data-id="${listener.id}">Edit</button>
                </div>
            `;

            container.appendChild(card);
        });
    } catch (err) {
        document.getElementById('listeners-list').textContent = 'Error fetching listeners';
        console.error(err);
    }
}

async function editListener(id) {
    const res = await fetch(`/listeners/${id}`);
    const listener = await res.json();

    // Send swal
    Swal.fire({
        customClass: {
            confirmButton: 'swal-confirm'
        },
        title: 'Edit Listener ' + id,
        background: mantle,
        color: textColor,
        confirmButtonColor: accentColor,
        confirmButtonText: 'Update Listener',
        html: `
            <div class="swal2-html-container">
                <h4>Channel URL</h4>
                <input id="swal-channel" class="swal2-input" value="${listener.channel_url}">
                <h4>Webhook URL</h4>
                <input id="swal-webhook" class="swal2-input" value="${listener.webhook_url}">
                <h4>Poll Interval (in seconds)</h4>
                <input id="swal-interval" class="swal2-input" type="number" value="${listener.poll_interval}">
            </div>
        `,
        // Prepare form
        preConfirm: () => ({
            id,
            channel_url: document.getElementById('swal-channel').value,
            webhook_url: document.getElementById('swal-webhook').value,
            poll_interval: parseInt(document.getElementById('swal-interval').value),
        })
    }).then(result => {
        if (result.isConfirmed) {
            fetch(`/listeners/${id}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(result.value)
            });
        }
    });
}

async function addListener() {
    Swal.fire({
        customClass: {
            confirmButton: 'swal-confirm'
        },
        title: 'Create Listener',
        background: mantle,
        color: textColor,
        confirmButtonColor: accentColor,
        confirmButtonText: 'Create',
        html: `
            <div class="swal2-html-container">
                <h4>Listener ID</h4>
                <input id="swal-id" class="swal2-input" placeholder="unique id">
                <h4>Channel URL</h4>
                <input id="swal-channel" class="swal2-input" placeholder="https://t.me/s/...">
                <h4>Webhook URL</h4>
                <input id="swal-webhook" class="swal2-input" placeholder="https://...">
                <h4>Poll Interval (in seconds)</h4>
                <input id="swal-interval" class="swal2-input" type="number" placeholder="67">
            </div>
        `,
        // Prepare form
        preConfirm: () => ({
            id: document.getElementById('swal-id').value,
            channel_url: document.getElementById('swal-channel').value,
            webhook_url: document.getElementById('swal-webhook').value,
            poll_interval: parseInt(document.getElementById('swal-interval').value),
        })
    }).then(result => {
        if (result.isConfirmed) {
            fetch(`/listeners`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(result.value)
            });
        }
    });
}

// Control buttons listener
document.getElementById('listeners-container').addEventListener('click', async (e) => {
    const btn = e.target.closest('button.listener-control');
    if (!btn) return;

    const action = btn.dataset.action;
    const id = btn.dataset.id;

    if (action === 'delete') {
        await fetch(`/listeners/${id}`, { method: 'DELETE' });
    } else if (action === 'edit') {
        await editListener(id);
    } else if (action === 'add') {
        await addListener();
    }
});

fetchListeners();
setInterval(fetchListeners, 2500);
