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
    if (!schema) return [];
    const props = schema.properties ?? {};
    const required = schema.required ?? [];
    return Object.entries(props).map(([id, def]) => ({
        id,
        label: def.title ?? id,
        type: def.type === 'integer' ? 'number' : 'text',
        required: required.includes(id),
    }));
}

function buildSwalFields(fields, existing = null) {
    return fields.map(f => `
        <div class="swal-field">
            <h4>${f.label}</h4>
            <input 
                id="swal-${f.id}" 
                class="swal2-input" 
                type="${f.type}" 
                placeholder="${f.label} (${f.type})" 
                value="${existing?.raw?.[f.id] ?? ''}"
            >
        </div>
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

            const def = SOURCE_TYPES.find(t => t.kind === source.kind);
            const fields = schemaToFields(def?.fields).filter(f => f.id !== 'id');
            const raw = source.raw ?? {};

            card.innerHTML = `
                <div class="card-header">
                    <h3 class="source-id">${source.id}</h3>
                    <div class="source-tags">
                        <span class="source-tag">${source.active ? 'Running' : 'Not Running'}</span>
                        <span class="source-tag">${def?.name ?? source.kind}</span>
                    </div>
                </div>
                <hr>

                <div class="card-body">
                    ${fields.map(f => `
                        <p class="source-attribute">${f.label}: ${raw[f.id] ?? '—'}</p>
                    `).join('')}
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
    const def = SOURCE_TYPES.find(t => t.kind === source.kind);
    const fields = schemaToFields(def?.fields).filter(f => f.id !== 'id');

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
        html: `<div class="swal2-html-container">${buildSwalFields(fields, source)}</div>`,
        // Prepare form
        preConfirm: () => {
            const raw = {};
            for (const f of fields) {
                const el  = document.getElementById(`swal-${f.id}`);
                raw[f.id] = f.type === 'number' ? parseInt(el.value) : el.value;
            }
            return { id, kind: source.kind, raw };
        }
    }).then(result => {
        if (result.isConfirmed) {
            fetch(`/sources/${id}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(result.value)
            }).then(() => fetchSources());
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
                <h4>Source Type</h4>
                <select id="swal-kind" class="swal2-select">
                    ${SOURCE_TYPES.map(t => `<option value="${t.kind}">${t.name}</option>`).join('')}
                </select>

                <div id="swal-fields">${buildSwalFields(schemaToFields(SOURCE_TYPES[0]?.fields))}</div>
            </div>
        `,
        didOpen: () => {
            // Listener to kind change
            document.getElementById('swal-kind').addEventListener('change', e => {
                const def = SOURCE_TYPES.find(t => t.kind === e.target.value);
                document.getElementById('swal-fields').innerHTML = buildSwalFields(schemaToFields(def?.fields));
            });
        },
        // Prepare form
        preConfirm: () => {
            const kind = document.getElementById('swal-kind').value;
            const def = SOURCE_TYPES.find(t => t.kind === kind);
            const fields = schemaToFields(def?.fields);
            const raw = {};

            for (const f of fields) {
                const el = document.getElementById(`swal-${f.id}`);
                raw[f.id] = f.type === 'number' ? parseInt(el.value) : el.value;
                if (f.required && !raw[f.id] && raw[f.id] !== 0) {
                    Swal.showValidationMessage(`${f.label} is required`);
                    return false;
                }
            }

            return { id: raw.id, kind, raw };
        }
    }).then(result => {
        if (result.isConfirmed) {
            fetch(`/sources`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(result.value)
            }).then(() => fetchSources());
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
        fetchSources();
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
    setInterval(fetchSources, 5000);
    setInterval(health, 5000);
});
