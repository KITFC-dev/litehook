const style = getComputedStyle(document.documentElement);
const bgColor = style.getPropertyValue('--bg-color').trim();
const mantle = style.getPropertyValue('--mantle').trim();
const textColor = style.getPropertyValue('--text-color').trim();
const accentColor = style.getPropertyValue('--accent-color').trim();

let SOURCE_TYPES = [];

async function loadSourceTypes() {
    const res = await fetch('/sources/types');
    SOURCE_TYPES = await res.json();
}

function schemaToFields(schema) {
    const props = schema?.properties ?? {};
    const required = schema?.required ?? [];
    return Object.entries(props).map(([id, def]) => ({
        id,
        label: def.title ?? id,
        type: def.type === 'integer' ? 'number' : 'text',
        required: required.includes(id),
    }));
}

function buildSwalFields(fields, existing = null) {
    return fields.map(f => `
        <h4>${f.label}</h4>
        <input id="swal-${f.id}" class="swal2-input" type="${f.type}" placeholder="${f.label}" value="${existing?.raw?.[f.id] ?? ''}">
    `).join('');
}

async function health() {
    try {
        const res = await fetch('/health');
        const data = await res.json();
        const sourcesCount = document.getElementById('health-sources-count');
        sourcesCount.textContent = `Running ${data.sources} source${data.sources !== 1 ? 's' : ''}`;
        sourcesCount.style.color = data.ok ? 'var(--success-color)' : 'var(--error-color)';
    } catch (err) {
        const sourcesCount = document.getElementById('health-sources-count');
        sourcesCount.textContent = `Could not connect to the litehook server.`;
        sourcesCount.style.color = 'var(--error-color)';
        console.error(err);
    }    
}

async function fetchSources() {
    try {
        const res = await fetch('/sources');
        const data = await res.json();
        const container = document.getElementById('sources-list');
        container.innerHTML = '';

        // List sources
        data.forEach(source => {
            const card = document.createElement('div');
            card.className = 'source-card';
            
            card.innerHTML = `
                <div class="card-header">
                    <h3 class="source-id">${source.id}</h3>
                    <div class="source-tags">
                        <span class="source-tag">${source.active ? 'Running' : 'Not Running'}</span>
                        <span class="source-tag">Interval: ${source.poll_interval} s</span>
                    </div>
                </div>
                <hr>
                <div class="card-body">
                    <p class="source-attribute">Channel URL: <a href="${source.channel_url}">${source.channel_url}</a></p>
                    <p class="source-attribute">Webhook URL: <a href="${source.webhook_url}">${source.webhook_url}</a></p>
                </div>
                <div class="source-footer">
                    <h4>Controls</h4>
                    <div class="source-controls">
                        <button class="source-control info" data-action="edit" data-id="${source.id}">
                            <i data-lucide="pencil"></i> Edit
                        </button>
                        <button class="source-control error" data-action="delete" data-id="${source.id}">
                            <i data-lucide="trash"></i> Delete
                        </button>
                    </div>
                </div>
            `;

            container.appendChild(card);
        });
        
        // Call after html is added
        lucide.createIcons();
    } catch (err) {
        const container = document.getElementById('sources-list');
        container.textContent = 'Error fetching sources';
        container.style.color = 'var(--error-color)';

        console.error(err);
    }
}

async function editSource(id) {
    const res = await fetch(`/sources/${id}`);
    const source = await res.json();

    // Send swal
    Swal.fire({
        customClass: {
            confirmButton: 'swal-confirm'
        },
        title: 'Edit source ' + id,
        background: mantle,
        color: textColor,
        confirmButtonColor: accentColor,
        confirmButtonText: 'Update source',
        html: `
            <div class="swal2-html-container">
                <h4>Channel URL</h4>
                <input id="swal-channel" class="swal2-input" value="${source.channel_url}">
                <h4>Webhook URL</h4>
                <input id="swal-webhook" class="swal2-input" value="${source.webhook_url}">
                <h4>Poll Interval (in seconds)</h4>
                <input id="swal-interval" class="swal2-input" type="number" value="${source.poll_interval}">
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
            fetch(`/sources/${id}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(result.value)
            });
        }
    });
}

async function addSource() {
    Swal.fire({
        customClass: {
            confirmButton: 'swal-confirm'
        },
        title: 'Add new source',
        background: mantle,
        color: textColor,
        confirmButtonColor: accentColor,
        confirmButtonText: 'Create',
        html: `
            <div class="swal2-html-container">
                <h4>Source ID</h4>
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
            fetch(`/sources`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(result.value)
            });
        }
    });
}

// Control buttons listener
document.getElementById('sources-container').addEventListener('click', async (e) => {
    const btn = e.target.closest('button.source-control');
    if (!btn) return;

    const action = btn.dataset.action;
    const id = btn.dataset.id;

    if (action === 'delete') {
        await fetch(`/sources/${id}`, { method: 'DELETE' });
    } else if (action === 'edit') {
        await editSource(id);
    } else if (action === 'add') {
        await addSource();
    }
});

document.addEventListener('DOMContentLoaded', async () => {
    await loadSourceTypes();
    fetchSources();
    health();
    setInterval(fetchSources, 2500);
    setInterval(health, 2500);
});
