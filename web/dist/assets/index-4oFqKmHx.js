var u=Object.defineProperty;var h=(o,e,t)=>e in o?u(o,e,{enumerable:!0,configurable:!0,writable:!0,value:t}):o[e]=t;var d=(o,e,t)=>h(o,typeof e!="symbol"?e+"":e,t);(function(){const e=document.createElement("link").relList;if(e&&e.supports&&e.supports("modulepreload"))return;for(const s of document.querySelectorAll('link[rel="modulepreload"]'))n(s);new MutationObserver(s=>{for(const i of s)if(i.type==="childList")for(const a of i.addedNodes)a.tagName==="LINK"&&a.rel==="modulepreload"&&n(a)}).observe(document,{childList:!0,subtree:!0});function t(s){const i={};return s.integrity&&(i.integrity=s.integrity),s.referrerPolicy&&(i.referrerPolicy=s.referrerPolicy),s.crossOrigin==="use-credentials"?i.credentials="include":s.crossOrigin==="anonymous"?i.credentials="omit":i.credentials="same-origin",i}function n(s){if(s.ep)return;s.ep=!0;const i=t(s);fetch(s.href,i)}})();class m{constructor(){d(this,"currentOwner",null);d(this,"currentRepo",null);d(this,"activeTab","files");d(this,"currentPath","");d(this,"currentRevision","main");this.init()}init(){window.addEventListener("popstate",()=>this.handleRoute()),this.handleRoute()}navigate(e){window.history.pushState({},"",e),this.handleRoute()}async handleRoute(){const t=window.location.pathname.split("/").filter(Boolean);t.length>=2&&t[0]!=="ui"?(this.currentOwner=t[0],this.currentRepo=t[1].replace(".git",""),await this.renderRepositoryPage()):t.length>=3&&t[0]==="ui"?(this.currentOwner=t[1],this.currentRepo=t[2].replace(".git",""),await this.renderRepositoryPage()):(this.currentOwner=null,this.currentRepo=null,await this.renderDashboard())}async renderDashboard(){var t,n,s;const e=document.getElementById("app");if(e){e.innerHTML=`
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
    `,(t=document.getElementById("btn-create-modal"))==null||t.addEventListener("click",()=>{var i;(i=document.getElementById("modal"))==null||i.classList.remove("hidden")}),(n=document.getElementById("btn-cancel-modal"))==null||n.addEventListener("click",()=>{var i;(i=document.getElementById("modal"))==null||i.classList.add("hidden")}),(s=document.getElementById("btn-submit-repo"))==null||s.addEventListener("click",()=>this.handleCreateRepo());try{const a=await(await fetch("/repositories")).json(),r=document.getElementById("repo-list");if(!r)return;if(a.length===0){r.innerHTML=`
          <div class="empty-state">
            <div class="empty-icon">📭</div>
            <div class="empty-text">No repositories yet. Create your first repository or push via CLI.</div>
          </div>
        `;return}r.innerHTML=a.map(c=>`
        <div class="repo-card" onclick="window.treeApp.navigate('/${c.owner_name}/${c.name}')">
          <div class="repo-card-header">
            <span class="repo-name">${c.owner_name} / <strong>${c.name}</strong></span>
            <span class="badge ${c.is_private?"badge-private":"badge-public"}">${c.is_private?"private":"public"}</span>
          </div>
          <div class="repo-desc">${c.description||"No description provided."}</div>
          <div class="repo-meta">Default branch: <code>${c.default_branch}</code> • Created ${new Date(c.created_at).toLocaleDateString()}</div>
        </div>
      `).join("")}catch(i){console.error(i)}}}async handleCreateRepo(){var s,i,a;const e=((s=document.getElementById("new-repo-owner"))==null?void 0:s.value)||"user",t=(i=document.getElementById("new-repo-name"))==null?void 0:i.value,n=(a=document.getElementById("new-repo-desc"))==null?void 0:a.value;if(!t){alert("Repository name is required");return}try{const r=await fetch("/repositories",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({owner:e,name:t,description:n})});if(r.ok)this.navigate(`/${e}/${t}`);else{const c=await r.text();alert(`Failed to create repository: ${c}`)}}catch(r){alert(`Network error: ${r}`)}}async renderRepositoryPage(){var t,n,s;const e=document.getElementById("app");if(!(!e||!this.currentOwner||!this.currentRepo))try{const i=await fetch(`/repositories/${this.currentOwner}/${this.currentRepo}`);if(!i.ok){e.innerHTML=`
          <div class="container">
            <div class="error-box">Repository ${this.currentOwner}/${this.currentRepo} not found.</div>
            <button class="btn" onclick="window.treeApp.navigate('/')">← Return Home</button>
          </div>
        `;return}const a=await i.json();this.currentRevision=a.default_branch,e.innerHTML=`
        <div class="header">
          <div class="breadcrumb">
            <span class="logo clickable" onclick="window.treeApp.navigate('/')">🌲 Tree</span>
            <span class="separator">/</span>
            <span class="owner-crumb">${this.currentOwner}</span>
            <span class="separator">/</span>
            <strong class="repo-crumb">${this.currentRepo}</strong>
            <span class="badge ${a.repository.is_private?"badge-private":"badge-public"}">${a.repository.is_private?"private":"public"}</span>
          </div>
        </div>

        <div class="container">
          <div class="clone-box">
            <div class="clone-label">Clone</div>
            <input type="text" readonly value="${a.clone_url_http}" class="clone-input" onclick="this.select()" />
            <button class="btn btn-sm" onclick="navigator.clipboard.writeText('${a.clone_url_http}')">Copy</button>
          </div>

          <div class="tabs">
            <button class="tab-btn ${this.activeTab==="files"?"active":""}" id="tab-files">Files</button>
            <button class="tab-btn ${this.activeTab==="branches"?"active":""}" id="tab-branches">Branches (${a.branches_count})</button>
            <button class="tab-btn ${this.activeTab==="commits"?"active":""}" id="tab-commits">Commits (${a.commits_count})</button>
          </div>

          <div id="tab-content" class="tab-content">
            <!-- Rendered Tab -->
          </div>
        </div>
      `,(t=document.getElementById("tab-files"))==null||t.addEventListener("click",()=>{this.activeTab="files",this.updateTabs(),this.renderFilesTab(a)}),(n=document.getElementById("tab-branches"))==null||n.addEventListener("click",()=>{this.activeTab="branches",this.updateTabs(),this.renderBranchesTab()}),(s=document.getElementById("tab-commits"))==null||s.addEventListener("click",()=>{this.activeTab="commits",this.updateTabs(),this.renderCommitsTab()}),this.activeTab==="files"?await this.renderFilesTab(a):this.activeTab==="branches"?await this.renderBranchesTab():await this.renderCommitsTab()}catch(i){console.error(i)}}updateTabs(){["files","branches","commits"].forEach(e=>{const t=document.getElementById(`tab-${e}`);t&&(e===this.activeTab?t.classList.add("active"):t.classList.remove("active"))})}async renderFilesTab(e){const t=document.getElementById("tab-content");if(t){if(e.is_empty){t.innerHTML=`
        <div class="empty-repo-instructions">
          <h3>Quick Setup</h3>
          <p>Get started by pushing from your local terminal:</p>
          <pre><code>git clone ${e.clone_url_http}
cd ${e.repository.name}
echo "# ${e.repository.name}" > README.md
git add README.md
git commit -m "initial commit"
git push -u origin main</code></pre>
        </div>
      `;return}try{const n=`/repositories/${this.currentOwner}/${this.currentRepo}/tree?ref=${this.currentRevision}&path=${encodeURIComponent(this.currentPath)}`,i=await(await fetch(n)).json();let a=`
        <div class="file-tree-card">
          <div class="tree-header">
            <span class="branch-pill">branch: <strong>${this.currentRevision}</strong></span>
            <span class="path-pill">${this.currentPath||"/"}</span>
          </div>
          <table class="tree-table">
            <tbody>
      `;if(this.currentPath){const r=this.currentPath.split("/").slice(0,-1).join("/");a+=`
          <tr class="tree-row back-row" onclick="window.treeApp.browsePath('${r}')">
            <td colspan="3"><span class="icon">📁</span> ..</td>
          </tr>
        `}for(const r of i)r.is_dir?a+=`
            <tr class="tree-row" onclick="window.treeApp.browsePath('${r.path}')">
              <td class="name-cell"><span class="icon">📁</span> <strong>${r.name}</strong></td>
              <td class="size-cell">-</td>
              <td class="action-cell">dir</td>
            </tr>
          `:a+=`
            <tr class="tree-row" onclick="window.treeApp.viewBlob('${r.path}')">
              <td class="name-cell"><span class="icon">📄</span> ${r.name}</td>
              <td class="size-cell">${p(r.size)}</td>
              <td class="action-cell">blob</td>
            </tr>
          `;a+=`
            </tbody>
          </table>
        </div>
      `,e.readme_content&&!this.currentPath&&(a+=`
          <div class="readme-card">
            <div class="readme-header">📖 README</div>
            <pre class="readme-body"><code>${l(e.readme_content)}</code></pre>
          </div>
        `),t.innerHTML=a}catch(n){t.innerHTML=`<div class="error-box">Failed to load files: ${n}</div>`}}}async browsePath(e){this.currentPath=e;const n=await(await fetch(`/repositories/${this.currentOwner}/${this.currentRepo}`)).json();await this.renderFilesTab(n)}async viewBlob(e){const t=document.getElementById("tab-content");if(t)try{const n=`/repositories/${this.currentOwner}/${this.currentRepo}/blob?ref=${this.currentRevision}&path=${encodeURIComponent(e)}`,i=await(await fetch(n)).json();t.innerHTML=`
        <div class="blob-card">
          <div class="blob-header">
            <button class="btn btn-sm" onclick="window.treeApp.browsePath('${this.currentPath}')">← Back to tree</button>
            <span class="blob-path">${i.path} (${p(i.size)})</span>
          </div>
          <div class="blob-body">
            ${i.is_binary?'<div class="binary-warning">Binary file cannot be displayed inline.</div>':`<pre><code>${l(i.content||"")}</code></pre>`}
          </div>
        </div>
      `}catch(n){t.innerHTML=`<div class="error-box">Failed to load file: ${n}</div>`}}async renderBranchesTab(){const e=document.getElementById("tab-content");if(e)try{const n=await(await fetch(`/repositories/${this.currentOwner}/${this.currentRepo}/branches`)).json();if(n.length===0){e.innerHTML='<div class="empty-state">No branches found.</div>';return}e.innerHTML=`
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
              ${n.map(s=>`
                <tr>
                  <td>
                    <strong>${s.name}</strong>
                    ${s.is_default?'<span class="badge badge-default">default</span>':""}
                  </td>
                  <td><code>${s.commit_id.slice(0,7)}</code></td>
                  <td>${s.commit_author||"-"}</td>
                  <td class="msg-cell">${s.commit_message||"-"}</td>
                </tr>
              `).join("")}
            </tbody>
          </table>
        </div>
      `}catch(t){e.innerHTML=`<div class="error-box">Failed to load branches: ${t}</div>`}}async renderCommitsTab(){const e=document.getElementById("tab-content");if(e)try{const n=await(await fetch(`/repositories/${this.currentOwner}/${this.currentRepo}/commits?ref=${this.currentRevision}`)).json();if(n.length===0){e.innerHTML='<div class="empty-state">No commits in this repository yet.</div>';return}e.innerHTML=`
        <div class="commits-card">
          <div class="commit-log">
            ${n.map(s=>`
              <div class="commit-item">
                <div class="commit-summary">
                  <strong>${l(s.summary)}</strong>
                </div>
                <div class="commit-meta">
                  <span class="author">${l(s.author_name)}</span> committed on
                  <span class="date">${new Date(s.timestamp).toLocaleString()}</span>
                  <span class="sha"><code>${s.short_id}</code></span>
                </div>
              </div>
            `).join("")}
          </div>
        </div>
      `}catch(t){e.innerHTML=`<div class="error-box">Failed to load commits: ${t}</div>`}}}function l(o){return o.replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;").replace(/"/g,"&quot;").replace(/'/g,"&#039;")}function p(o){if(o===0)return"0 B";const e=1024,t=["B","KB","MB","GB"],n=Math.floor(Math.log(o)/Math.log(e));return parseFloat((o/Math.pow(e,n)).toFixed(1))+" "+t[n]}window.treeApp=new m;
