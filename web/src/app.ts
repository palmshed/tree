// Tree Minimal Web Interface TypeScript Client

interface Repository {
  id: string;
  owner_name: string;
  name: string;
  description: string | null;
  is_private: boolean;
  default_branch: string;
  created_at: string;
}

interface RepositorySummary {
  repository: Repository;
  default_branch: string;
  branches_count: number;
  tags_count: number;
  commits_count: number;
  is_empty: boolean;
  clone_url_http: string;
  clone_url_ssh: string;
  readme_content: string | null;
}

interface BranchInfo {
  name: string;
  commit_id: string;
  is_default: boolean;
  commit_message: string | null;
  commit_author: string | null;
  commit_date: string | null;
}

interface CommitInfo {
  id: string;
  short_id: string;
  author_name: string;
  author_email: string;
  message: string;
  summary: string;
  timestamp: string;
  parents: string[];
}

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  mode: number;
}

interface FileContent {
  path: string;
  name: string;
  size: number;
  is_binary: boolean;
  content: string | null;
}

class TreeApp {
  private currentOwner: string | null = null;
  private currentRepo: string | null = null;
  private activeTab: 'files' | 'branches' | 'commits' = 'files';
  private currentPath: string = '';
  private currentRevision: string = 'main';

  constructor() {
    this.init();
  }

  private init() {
    window.addEventListener('popstate', () => this.handleRoute());
    this.handleRoute();
  }

  public navigate(url: string) {
    window.history.pushState({}, '', url);
    this.handleRoute();
  }

  private async handleRoute() {
    const path = window.location.pathname;
    const parts = path.split('/').filter(Boolean);

    if (parts.length >= 2 && parts[0] !== 'ui') {
      this.currentOwner = parts[0];
      this.currentRepo = parts[1].replace('.git', '');
      await this.renderRepositoryPage();
    } else if (parts.length >= 3 && parts[0] === 'ui') {
      this.currentOwner = parts[1];
      this.currentRepo = parts[2].replace('.git', '');
      await this.renderRepositoryPage();
    } else {
      this.currentOwner = null;
      this.currentRepo = null;
      await this.renderDashboard();
    }
  }

  private async renderDashboard() {
    const appEl = document.getElementById('app');
    if (!appEl) return;

    appEl.innerHTML = `
      <div class="header">
        <div class="logo-title">
          <span class="logo">🌲</span>
          <span class="site-name">Tree</span>
          <span class="subtitle">Quiet Git Hosting</span>
        </div>
      </div>

      <div class="container">
        <div class="toolbar">
          <div class="section-title">Repositories</div>
          <button class="btn btn-primary" id="btn-create-modal">+ New Repository</button>
        </div>

        <div id="repo-list" class="repo-list">
          <div class="loading">Loading repositories...</div>
        </div>
      </div>

      <div id="modal" class="modal hidden">
        <div class="modal-content">
          <div class="modal-header">Create Repository</div>
          <div class="form-group">
            <label>Owner</label>
            <input type="text" id="new-repo-owner" value="user" class="input" />
          </div>
          <div class="form-group">
            <label>Name</label>
            <input type="text" id="new-repo-name" placeholder="my-awesome-project" class="input" />
          </div>
          <div class="form-group">
            <label>Description (optional)</label>
            <input type="text" id="new-repo-desc" placeholder="A quiet project" class="input" />
          </div>
          <div class="modal-actions">
            <button class="btn" id="btn-cancel-modal">Cancel</button>
            <button class="btn btn-primary" id="btn-submit-repo">Create</button>
          </div>
        </div>
      </div>
    `;

    document.getElementById('btn-create-modal')?.addEventListener('click', () => {
      document.getElementById('modal')?.classList.remove('hidden');
    });

    document.getElementById('btn-cancel-modal')?.addEventListener('click', () => {
      document.getElementById('modal')?.classList.add('hidden');
    });

    document.getElementById('btn-submit-repo')?.addEventListener('click', () => this.handleCreateRepo());

    try {
      const res = await fetch('/repositories');
      const repos: Repository[] = await res.json();
      const listEl = document.getElementById('repo-list');
      if (!listEl) return;

      if (repos.length === 0) {
        listEl.innerHTML = `
          <div class="empty-state">
            <div class="empty-icon">📭</div>
            <div class="empty-text">No repositories yet. Create your first repository or push via CLI.</div>
          </div>
        `;
        return;
      }

      listEl.innerHTML = repos
        .map(
          (r) => `
        <div class="repo-card" onclick="window.treeApp.navigate('/${r.owner_name}/${r.name}')">
          <div class="repo-card-header">
            <span class="repo-name">${r.owner_name} / <strong>${r.name}</strong></span>
            <span class="badge ${r.is_private ? 'badge-private' : 'badge-public'}">${
            r.is_private ? 'private' : 'public'
          }</span>
          </div>
          <div class="repo-desc">${r.description || 'No description provided.'}</div>
          <div class="repo-meta">Default branch: <code>${r.default_branch}</code> • Created ${new Date(
            r.created_at
          ).toLocaleDateString()}</div>
        </div>
      `
        )
        .join('');
    } catch (err) {
      console.error(err);
    }
  }

  private async handleCreateRepo() {
    const owner = (document.getElementById('new-repo-owner') as HTMLInputElement)?.value || 'user';
    const name = (document.getElementById('new-repo-name') as HTMLInputElement)?.value;
    const description = (document.getElementById('new-repo-desc') as HTMLInputElement)?.value;

    if (!name) {
      alert('Repository name is required');
      return;
    }

    try {
      const res = await fetch('/repositories', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ owner, name, description }),
      });

      if (res.ok) {
        this.navigate(`/${owner}/${name}`);
      } else {
        const err = await res.text();
        alert(`Failed to create repository: ${err}`);
      }
    } catch (e) {
      alert(`Network error: ${e}`);
    }
  }

  private async renderRepositoryPage() {
    const appEl = document.getElementById('app');
    if (!appEl || !this.currentOwner || !this.currentRepo) return;

    try {
      const res = await fetch(`/repositories/${this.currentOwner}/${this.currentRepo}`);
      if (!res.ok) {
        appEl.innerHTML = `
          <div class="container">
            <div class="error-box">Repository ${this.currentOwner}/${this.currentRepo} not found.</div>
            <button class="btn" onclick="window.treeApp.navigate('/')">← Return Home</button>
          </div>
        `;
        return;
      }

      const summary: RepositorySummary = await res.json();
      this.currentRevision = summary.default_branch;

      appEl.innerHTML = `
        <div class="header">
          <div class="breadcrumb">
            <span class="logo clickable" onclick="window.treeApp.navigate('/')">🌲 Tree</span>
            <span class="separator">/</span>
            <span class="owner-crumb">${this.currentOwner}</span>
            <span class="separator">/</span>
            <strong class="repo-crumb">${this.currentRepo}</strong>
            <span class="badge ${summary.repository.is_private ? 'badge-private' : 'badge-public'}">${
        summary.repository.is_private ? 'private' : 'public'
      }</span>
          </div>
        </div>

        <div class="container">
          <div class="clone-box">
            <div class="clone-label">Clone</div>
            <input type="text" readonly value="${summary.clone_url_http}" class="clone-input" onclick="this.select()" />
            <button class="btn btn-sm" onclick="navigator.clipboard.writeText('${summary.clone_url_http}')">Copy</button>
          </div>

          <div class="tabs">
            <button class="tab-btn ${this.activeTab === 'files' ? 'active' : ''}" id="tab-files">Files</button>
            <button class="tab-btn ${this.activeTab === 'branches' ? 'active' : ''}" id="tab-branches">Branches (${summary.branches_count})</button>
            <button class="tab-btn ${this.activeTab === 'commits' ? 'active' : ''}" id="tab-commits">Commits (${summary.commits_count})</button>
          </div>

          <div id="tab-content" class="tab-content">
            <!-- Rendered Tab -->
          </div>
        </div>
      `;

      document.getElementById('tab-files')?.addEventListener('click', () => {
        this.activeTab = 'files';
        this.updateTabs();
        this.renderFilesTab(summary);
      });

      document.getElementById('tab-branches')?.addEventListener('click', () => {
        this.activeTab = 'branches';
        this.updateTabs();
        this.renderBranchesTab();
      });

      document.getElementById('tab-commits')?.addEventListener('click', () => {
        this.activeTab = 'commits';
        this.updateTabs();
        this.renderCommitsTab();
      });

      if (this.activeTab === 'files') {
        await this.renderFilesTab(summary);
      } else if (this.activeTab === 'branches') {
        await this.renderBranchesTab();
      } else {
        await this.renderCommitsTab();
      }
    } catch (e) {
      console.error(e);
    }
  }

  private updateTabs() {
    ['files', 'branches', 'commits'].forEach((tab) => {
      const btn = document.getElementById(`tab-${tab}`);
      if (btn) {
        if (tab === this.activeTab) {
          btn.classList.add('active');
        } else {
          btn.classList.remove('active');
        }
      }
    });
  }

  private async renderFilesTab(summary: RepositorySummary) {
    const container = document.getElementById('tab-content');
    if (!container) return;

    if (summary.is_empty) {
      container.innerHTML = `
        <div class="empty-repo-instructions">
          <h3>Quick Setup</h3>
          <p>Get started by pushing from your local terminal:</p>
          <pre><code>git clone ${summary.clone_url_http}
cd ${summary.repository.name}
echo "# ${summary.repository.name}" > README.md
git add README.md
git commit -m "initial commit"
git push -u origin main</code></pre>
        </div>
      `;
      return;
    }

    try {
      const url = `/repositories/${this.currentOwner}/${this.currentRepo}/tree?ref=${this.currentRevision}&path=${encodeURIComponent(
        this.currentPath
      )}`;
      const res = await fetch(url);
      const entries: FileEntry[] = await res.json();

      let html = `
        <div class="file-tree-card">
          <div class="tree-header">
            <span class="branch-pill">branch: <strong>${this.currentRevision}</strong></span>
            <span class="path-pill">${this.currentPath || '/'}</span>
          </div>
          <table class="tree-table">
            <tbody>
      `;

      if (this.currentPath) {
        const parentPath = this.currentPath.split('/').slice(0, -1).join('/');
        html += `
          <tr class="tree-row back-row" onclick="window.treeApp.browsePath('${parentPath}')">
            <td colspan="3"><span class="icon">📁</span> ..</td>
          </tr>
        `;
      }

      for (const entry of entries) {
        if (entry.is_dir) {
          html += `
            <tr class="tree-row" onclick="window.treeApp.browsePath('${entry.path}')">
              <td class="name-cell"><span class="icon">📁</span> <strong>${entry.name}</strong></td>
              <td class="size-cell">-</td>
              <td class="action-cell">dir</td>
            </tr>
          `;
        } else {
          html += `
            <tr class="tree-row" onclick="window.treeApp.viewBlob('${entry.path}')">
              <td class="name-cell"><span class="icon">📄</span> ${entry.name}</td>
              <td class="size-cell">${formatBytes(entry.size)}</td>
              <td class="action-cell">blob</td>
            </tr>
          `;
        }
      }

      html += `
            </tbody>
          </table>
        </div>
      `;

      if (summary.readme_content && !this.currentPath) {
        html += `
          <div class="readme-card">
            <div class="readme-header">📖 README</div>
            <pre class="readme-body"><code>${escapeHtml(summary.readme_content)}</code></pre>
          </div>
        `;
      }

      container.innerHTML = html;
    } catch (e) {
      container.innerHTML = `<div class="error-box">Failed to load files: ${e}</div>`;
    }
  }

  public async browsePath(path: string) {
    this.currentPath = path;
    const res = await fetch(`/repositories/${this.currentOwner}/${this.currentRepo}`);
    const summary: RepositorySummary = await res.json();
    await this.renderFilesTab(summary);
  }

  public async viewBlob(path: string) {
    const container = document.getElementById('tab-content');
    if (!container) return;

    try {
      const url = `/repositories/${this.currentOwner}/${this.currentRepo}/blob?ref=${this.currentRevision}&path=${encodeURIComponent(
        path
      )}`;
      const res = await fetch(url);
      const blob: FileContent = await res.json();

      container.innerHTML = `
        <div class="blob-card">
          <div class="blob-header">
            <button class="btn btn-sm" onclick="window.treeApp.browsePath('${this.currentPath}')">← Back to tree</button>
            <span class="blob-path">${blob.path} (${formatBytes(blob.size)})</span>
          </div>
          <div class="blob-body">
            ${
              blob.is_binary
                ? '<div class="binary-warning">Binary file cannot be displayed inline.</div>'
                : `<pre><code>${escapeHtml(blob.content || '')}</code></pre>`
            }
          </div>
        </div>
      `;
    } catch (e) {
      container.innerHTML = `<div class="error-box">Failed to load file: ${e}</div>`;
    }
  }

  private async renderBranchesTab() {
    const container = document.getElementById('tab-content');
    if (!container) return;

    try {
      const res = await fetch(`/repositories/${this.currentOwner}/${this.currentRepo}/branches`);
      const branches: BranchInfo[] = await res.json();

      if (branches.length === 0) {
        container.innerHTML = `<div class="empty-state">No branches found.</div>`;
        return;
      }

      container.innerHTML = `
        <div class="branches-card">
          <table class="data-table">
            <thead>
              <tr>
                <th>Branch</th>
                <th>Commit</th>
                <th>Last Author</th>
                <th>Message</th>
              </tr>
            </thead>
            <tbody>
              ${branches
                .map(
                  (b) => `
                <tr>
                  <td>
                    <strong>${b.name}</strong>
                    ${b.is_default ? '<span class="badge badge-default">default</span>' : ''}
                  </td>
                  <td><code>${b.commit_id.slice(0, 7)}</code></td>
                  <td>${b.commit_author || '-'}</td>
                  <td class="msg-cell">${b.commit_message || '-'}</td>
                </tr>
              `
                )
                .join('')}
            </tbody>
          </table>
        </div>
      `;
    } catch (e) {
      container.innerHTML = `<div class="error-box">Failed to load branches: ${e}</div>`;
    }
  }

  private async renderCommitsTab() {
    const container = document.getElementById('tab-content');
    if (!container) return;

    try {
      const res = await fetch(
        `/repositories/${this.currentOwner}/${this.currentRepo}/commits?ref=${this.currentRevision}`
      );
      const commits: CommitInfo[] = await res.json();

      if (commits.length === 0) {
        container.innerHTML = `<div class="empty-state">No commits in this repository yet.</div>`;
        return;
      }

      container.innerHTML = `
        <div class="commits-card">
          <div class="commit-log">
            ${commits
              .map(
                (c) => `
              <div class="commit-item">
                <div class="commit-summary">
                  <strong>${escapeHtml(c.summary)}</strong>
                </div>
                <div class="commit-meta">
                  <span class="author">${escapeHtml(c.author_name)}</span> committed on
                  <span class="date">${new Date(c.timestamp).toLocaleString()}</span>
                  <span class="sha"><code>${c.short_id}</code></span>
                </div>
              </div>
            `
              )
              .join('')}
          </div>
        </div>
      `;
    } catch (e) {
      container.innerHTML = `<div class="error-box">Failed to load commits: ${e}</div>`;
    }
  }
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

// Global initialization
(window as any).treeApp = new TreeApp();
