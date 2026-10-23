'use strict';

// Display terminal cells emitted by rstop::ui::draw. No alternate product UI.
(() => {
  const NS = 'http://www.w3.org/2000/svg';
  const views = ['overview','processes','gpu','storage','network','system','containers','services'];
  const themes = ['midnight','nord','ember','black','white','graphite','ocean','forest','sand'];
  const sizeFor = width => width<650?'compact':width<1000?'medium':'wide';
  const cache = new Map();
  const motion = matchMedia('(prefers-reduced-motion: reduce)');
  const request = path => {
    if (!cache.has(path)) cache.set(path,fetch(path).then(response=> {
      if (!response.ok) throw new Error(`Could not load ${path}`);
      return response.json();
    }).catch(error=> {cache.delete(path);throw error;}));
    return cache.get(path);
  };
  const node = (name, attributes = {}, text) => {
    const element = document.createElementNS(NS,name);
    Object.entries(attributes).forEach(([key,value])=>element.setAttribute(key,String(value)));
    if (text !== undefined) element.textContent=text;
    return element;
  };
  const lines = {'─':'M0 9 H8','━':'M0 9 H8','│':'M4 0 V18','╭':'M4 18 V13 Q4 9 8 9','╮':'M0 9 Q4 9 4 13 V18','╰':'M4 0 V5 Q4 9 8 9','╯':'M4 0 V5 Q4 9 0 9'};
  let nextID = 0;
  const processQuery = query => {
    const allowed=['','pid','user','name','state','cmd','command','cpu','mem','memory','gpu','vram','tag'];
    let error='';
    const groups=query.split(' | ').map(group=> {
      const words=[];let word='',quoted=false;
      for (const char of group) {if(char==='"') quoted=!quoted;else if(/\s/.test(char)&&!quoted) {if(word){words.push(word);word='';}}else word+=char;}
      if(word)words.push(word);if(quoted)error='Unclosed quoted term';
      return words.map(word=> {
        const negated=word.startsWith('!');if(negated)word=word.slice(1);
        const colon=word.indexOf(':'),field=colon<0?'':word.slice(0,colon).toLowerCase(),value=colon<0?word:word.slice(colon+1);
        if(!allowed.includes(field))error=`Unknown field: ${field}`;
        let regex;try{if(value.startsWith('/')&&value.endsWith('/')&&value.length>=2)regex=new RegExp(value.slice(1,-1),'i');}catch{error='Invalid regular expression';}
        return {field,value:value.toLowerCase(),negated,regex};
      });
    });
    return {error,matches:process=>!error&&groups.some(group=>group.every(term=> {
      const tags=JSON.stringify(process.tags||[]),values={pid:process.pid,user:process.user,name:process.name,state:process.state,cmd:process.command,command:process.command,cpu:process.cpu,mem:process.memory/1048576,memory:process.memory/1048576,gpu:process.gpu_percent??'—',vram:process.gpu_memory==null?'—':process.gpu_memory/1048576,tag:tags,'':`${process.pid} ${process.user} ${process.name} ${process.command} ${tags}`};
      const value=String(values[term.field]??'');const numeric=term.value.match(/^([><=])(-?[\d.]+)$/);
      const match=numeric?Number.isFinite(Number(value))&&(numeric[1]==='>'?Number(value)>Number(numeric[2]):numeric[1]==='<'?Number(value)<Number(numeric[2]):Number(value)===Number(numeric[2])):term.regex?term.regex.test(value):value.toLowerCase().includes(term.value);
      return match!==term.negated;
    }))};
  };

  class NativeDemo extends HTMLElement {
    static get observedAttributes() { return ['view','theme']; }
    connectedCallback() {
      if (this.initialized) return;
      this.initialized=true;
      this.tabIndex=0;
      this.uid=`native-${++nextID}`;
      this.paused=motion.matches;
      this.frame=0;
      this.scenario='live';
      this.query='';this.sortKey=null;this.sortReverse=false;this.inspectedId=null;this.tree=false;
      this.generation=0;
      this.size=sizeFor(this.getBoundingClientRect().width);
      this.innerHTML=`<div class="native-stage"><span class="native-loading" role="status">Loading rstop…</span></div>
        <div class="native-table-tools" hidden><input type="search" aria-label="Filter rows" placeholder="Filter"><button type="button" data-native-action="tree" hidden aria-pressed="false">Tree</button><button type="button" data-native-action="close-details" hidden>Close details</button><span role="status" class="native-filter-status"></span></div>`;
      this.addEventListener('input',event=> {if(event.target.matches('input')){this.query=event.target.value;this.scenario='live';this.lastRows=[];this.draw();}});
      this.addEventListener('pointerdown',event=>{if(event.target.closest('.native-stage')&&!event.target.closest('[role="button"]'))this.focus({preventScroll:true});});
      this.addEventListener('click',event=> {
        const button=event.target.closest('[data-native-view],[data-native-action],[data-native-row],[data-native-sort]');
        if (!button) return;
        event.preventDefault();
        if (button.dataset.nativeView) {this.setView(button.dataset.nativeView);this.focus({preventScroll:true});}
        if (button.dataset.nativeRow) {this.inspectedId=button.dataset.nativeRow;this.scenario='live';this.lastRows=[];this.draw();this.focus({preventScroll:true});}
        if (button.dataset.nativeSort) {
          this.sortReverse=this.sortKey===button.dataset.nativeSort?!this.sortReverse:false;
          this.sortKey=button.dataset.nativeSort;this.scenario='live';this.lastRows=[];this.draw();this.focus({preventScroll:true});
        }
        if (button.dataset.nativeAction==='tree') {this.tree=!this.tree;this.inspectedId=null;this.scenario='live';this.lastRows=[];this.draw();}
        if (button.dataset.nativeAction==='close-details') {this.inspectedId=null;this.scenario='live';this.lastRows=[];this.draw();}
      });
      this.addEventListener('keydown',event=> {
        const key=event.key;
        if(event.target.matches('input')) {if(key==='Escape'){event.target.value='';this.query='';this.lastRows=[];this.draw();}return;}
        if(event.target.matches('button')&&(key==='Enter'||key===' '))return;
        if((key==='Enter'||key===' ')&&event.target.closest('[data-native-sort],[data-native-row]')){event.preventDefault();event.target.closest('[data-native-sort],[data-native-row]').dispatchEvent(new MouseEvent('click',{bubbles:true}));return;}
        if(key==='Escape'&&this.inspectedId){event.preventDefault();this.inspectedId=null;this.lastRows=[];this.draw();return;}
        if((key==='/'||key==='F3')&&!this.querySelector('.native-table-tools').hidden){event.preventDefault();this.querySelector('input').focus();return;}
        if(key==='F4'&&!this.querySelector('.native-table-tools').hidden){event.preventDefault();this.hits.querySelector('[data-native-sort]')?.focus();return;}
        if(key==='F5'){event.preventDefault();this.querySelector(`[data-native-view="${this.view}"]`)?.focus();return;}
        if(['ArrowDown','ArrowUp','Enter'].includes(key)&&this.filteredBindings?.length){
          event.preventDefault();const rows=this.filteredBindings,index=rows.findIndex(row=>row.id===this.inspectedId);
          this.inspectedId=rows[Math.max(0,Math.min(rows.length-1,index+(key==='ArrowDown'?1:key==='ArrowUp'?-1:0)))].id;
          this.lastRows=[];this.draw();this.focus({preventScroll:true});return;
        }
        if (event.target.closest('[data-native-view]') && (key==='Enter'||key===' ')) {event.preventDefault();this.setView(event.target.closest('[data-native-view]').dataset.nativeView);return;}
        let index;
        if (/^[1-8]$/.test(key)) index=Number(key)-1;
        else if (key==='ArrowRight') index=(views.indexOf(this.view)+1)%8;
        else if (key==='ArrowLeft') index=(views.indexOf(this.view)+7)%8;
        else if (key==='Home') index=0;
        else if (key==='End') index=7;
        if (index!==undefined) {event.preventDefault();event.stopPropagation();this.setView(views[index]);}
      });
      this.visibleObserver=new IntersectionObserver(entries=> {this.visible=entries[0].isIntersecting;});
      this.visibleObserver.observe(this);
      this.resizeObserver=new ResizeObserver(entries=> {
        const size=sizeFor(entries[0].contentRect.width);
        if (size!==this.size) {this.size=size;this.load();}
      });
      this.resizeObserver.observe(this);
      this.onMotion=()=> {this.paused=motion.matches;};
      motion.addEventListener('change',this.onMotion);
      this.timer=setInterval(()=> {
        if (!this.pack||this.pending||this.paused||!this.visible||document.hidden) return;
        const now=performance.now();
        if (now-(this.lastTick||0)<1000) return;
        this.lastTick=now;
        this.frame=(this.frame+1)%this.pack.sequences.live.length;
        this.draw();
      },200);
      this.load();
    }
    disconnectedCallback() {
      clearInterval(this.timer);this.visibleObserver?.disconnect();this.resizeObserver?.disconnect();
      motion.removeEventListener('change',this.onMotion);
    }
    attributeChangedCallback(name,before,after) {
      if (!this.initialized||before===after) return;
      if (name==='theme') this.applyTheme();else this.load();
    }
    get view() {return views.includes(this.getAttribute('view'))?this.getAttribute('view'):'overview';}
    setView(view) {
      this.setAttribute('view',view);
      this.dispatchEvent(new CustomEvent('native-view-change',{bubbles:true,detail:{view}}));
    }
    load() {
      const generation=++this.generation, view=this.view, size=this.size;
      this.pending=true;
      if(this.loadedView!==view){this.query='';this.sortKey=null;this.sortReverse=false;this.inspectedId=null;this.tree=false;this.querySelector('input').value='';this.loadedView=view;}
      this.loading=Promise.all([request(`assets/native/${view}-${size}.json`),request('assets/native/renderer.json')]).then(([pack,metadata])=> {
        if (generation!==this.generation) return;
        this.pack=pack;this.metadata=metadata;this.scenario='live';this.frame=0;this.lastRows=[];this.pending=false;
        const width=pack.width*8+32,height=pack.height*18+32;
        this.style.setProperty('--native-ratio',`${width} / ${height}`);
        this.svg=node('svg',{viewBox:`0 0 ${width} ${height}`,width,height,role:'group','aria-label':`rstop ${view} · native interface · demo data`,'xml:space':'preserve'});
        this.svg.append(node('rect',{width:'100%',height:'100%',fill:'var(--native-0)'}));
        this.styles=node('style');this.svg.append(this.styles);
        this.groups=Array.from({length:pack.height},(_,y)=>node('g',{transform:`translate(16 ${16+y*18})`}));
        this.svg.append(...this.groups);
        const links=node('g',{'aria-label':'Resource views'});
        pack.tabs.forEach(([x,y,w,h,key])=> {
          const link=node('g',{'data-native-view':key,role:'button',tabindex:'0','aria-label':`Show ${key}`});
          link.append(node('rect',{x:16+x*8,y:16+y*18,width:w*8,height:h*18,fill:'transparent',class:'native-hit'}));links.append(link);
        });
        this.svg.append(links);
        this.overlays=node('g');this.hits=node('g',{'aria-label':'Table controls'});this.svg.append(this.overlays,this.hits);
        this.querySelector('.native-stage').replaceChildren(this.svg);
        this.setAttribute('data-renderer',metadata.renderer);
        this.applyTheme();this.draw();
      }).catch(error=> {
        if (generation!==this.generation) return;
        this.querySelector('.native-stage').textContent='The demo could not load. Refresh to try again.';
        console.error(error);
      });
      return this.loading;
    }
    applyTheme() {
      if (!this.pack) return;
      const theme=themes.includes(this.getAttribute('theme'))?this.getAttribute('theme'):'midnight';
      const themeIndex=themes.indexOf(theme),palette=this.pack.palettes[theme],flags=this.metadata.flags;
      palette.forEach((color,index)=>this.style.setProperty(`--native-${index}`,color));
      const color=value=>typeof value==='number'?`var(--native-${value})`:value;
      this.styles.textContent=this.pack.styles.map((variants,id)=> {
        let [fg,bg,mod]=variants[themeIndex];
        if (mod&flags.reverse) [fg,bg]=[bg,fg];
        return `.${this.uid}-f${id}{fill:${color(fg)};${mod&flags.bold?'font-weight:bold;':''}${mod&flags.dim?'opacity:.6;':''}${mod&flags.italic?'font-style:italic;':''}${mod&flags.underline?'text-decoration:underline;':''}}.${this.uid}-f${id} .native-boxline{stroke:${color(fg)}}.${this.uid}-b${id}{fill:${color(bg)}}`;
      }).join('');
      this.dispatchEvent(new CustomEvent('native-theme-change',{bubbles:true,detail:{theme,colors:{
        surface:palette[1],border:palette[2],muted:palette[3],text:palette[4],accent:palette[5],selected:palette[10]
      }}}));
    }
    renderRow(id) {
      const fragment=document.createDocumentFragment();
      let column=0;
      this.pack.rows[id].forEach(([style,text])=> {
        const symbols=Array.from(text),length=symbols.length,x=column*8;
        fragment.append(node('rect',{x,y:0,width:length*8,height:18,class:`${this.uid}-b${style}`}));
        const group=node('g',{class:`${this.uid}-f${style}`});
        let run='',start=column;
        const flush=()=> {
          if (run.trim()) group.append(node('text',{x:start*8,y:14,textLength:Array.from(run).length*8,lengthAdjust:'spacingAndGlyphs'},run));
          run='';
        };
        symbols.forEach(symbol=> {
          const code=symbol.codePointAt(0),isBraille=code>=0x2800&&code<=0x28ff;
          if (isBraille||lines[symbol]) {
            flush();
            if (isBraille) {
              const dots=code-0x2800;
              [[0,2,2],[1,2,6],[2,2,10],[3,6,2],[4,6,6],[5,6,10],[6,2,14],[7,6,14]].forEach(([bit,dx,dy])=> {
                if (dots&(1<<bit)) group.append(node('circle',{cx:column*8+dx,cy:dy,r:.85}));
              });
            } else group.append(node('path',{transform:`translate(${column*8} 0)`,d:lines[symbol],fill:'none','stroke-width':symbol==='━'?2:1,class:'native-boxline'}));
            start=column+1;
          } else {if (!run) start=column;run+=symbol;}
          column+=1;
        });
        flush();fragment.append(group);
      });
      return fragment;
    }
    tableState() {
      const base=this.pack.sequences.live[this.frame%this.pack.sequences.live.length];
      let rows=[...(base.bindings||[])];
      const parsed=processQuery(this.query);
      if(this.query) rows=rows.filter(row=>this.view==='processes'?parsed.matches(row.model):JSON.stringify(row.model).toLowerCase().includes(this.query.toLowerCase()));
      this.querySelector('.native-filter-status').textContent=this.view==='processes'?parsed.error:'';
      const keys={CPU:'cpu',RAM:'memory',NAME:'name',PID:'pid',USER:'user',GPU:'gpu_percent',VRAM:'gpu_memory',DESTINATION:'remote','RTT ms':'rtt_ms',DOWN:'rx_per_sec',UP:'tx_per_sec',STATE:'state',PROTO:'protocol',LOCAL:'local',RECEIVED:'received',SENT:'sent'};
      const ascendingKeys=['NAME','PID','USER','STATE','PROCESS','DESTINATION','PROTO','LOCAL','LPORT','DPORT'];
      if(this.sortKey){
        const key=keys[this.sortKey]||this.sortKey.toLowerCase(),ascending=ascendingKeys.includes(this.sortKey);
        const value=model=>this.sortKey==='TOTAL/s'?(model.rx_per_sec??0)+(model.tx_per_sec??0):this.sortKey==='PROCESS'?model.process_names?.join(', '):model[key]??(key==='cpu'?model.cpu_percent:key==='pid'?model.pids?.[0]:key==='state'?model.active:undefined);
        rows.sort((a,b)=>{const av=value(a.model),bv=value(b.model);if(av==null||bv==null)return av==null?(bv==null?0:1):-1;const order=typeof av==='number'?av-bv:String(av).localeCompare(String(bv));return order*(ascending?1:-1)*(this.sortReverse?-1:1);});
      }
      if(this.inspectedId&&!rows.some(row=>row.id===this.inspectedId))this.inspectedId=null;
      const selected=(base.bindings||[]).findIndex(row=>row.id===this.inspectedId);
      let sequence=selected>=0?this.pack.sequences[(this.tree?'tree-':'')+'inspect-'+selected]:this.tree?this.pack.sequences.tree:this.pack.sequences.live;
      if(!rows.length&&this.pack.sequences.empty)sequence=this.pack.sequences.empty;
      const frame=(sequence||this.pack.sequences.live)[this.frame%(sequence||this.pack.sequences.live).length];
      this.filteredBindings=rows;
      return {frame,base,rows};
    }
    overlayText(x,y,width,text,foreground=4,background=1,bold=false) {
      if(width<=0)return;
      const value=Array.from(text).slice(0,width).join('');
      this.overlays.append(node('rect',{x:16+x*8,y:16+y*18,width:width*8,height:18,fill:`var(--native-${background})`}));
      if(value)this.overlays.append(node('text',{x:16+x*8,y:30+y*18,textLength:Array.from(value).length*8,lengthAdjust:'spacingAndGlyphs',fill:`var(--native-${foreground})`,'font-weight':bold?'bold':'normal'},value));
    }
    composeTable(frame,base,rows) {
      this.overlays.replaceChildren();this.hits.replaceChildren();
      if(!base.bindings?.length)return;
      const [x,y,width,height]=frame.table;
      const templates=new Map((frame.bindings||[]).map(binding=>[binding.id,frame.rows[binding.y]]));
      if(rows.length){
        this.overlays.append(node('rect',{x:16+x*8,y:16+y*18,width:width*8,height:height*18,fill:'var(--native-1)'}));
        rows.slice(0,height).forEach((row,index)=> {
          const source=templates.get(row.id);if(source===undefined)return;
          const target=node('svg',{x:16+x*8,y:16+(y+index)*18,width:width*8,height:18,viewBox:`${x*8} 0 ${width*8} 18`});
          target.append(this.renderRow(source));this.overlays.append(target);
          const hit=node('g',{'data-native-row':row.id,role:'button',tabindex:0,'aria-label':`Inspect ${row.model.name||row.model.process_names?.join(', ')||row.id}`});
          hit.append(node('rect',{x:16+x*8,y:16+(y+index)*18,width:width*8,height:18,fill:'transparent',class:'native-hit'}));this.hits.append(hit);
        });
      }
      const [fx,fy,fw]=frame.filter;
      if(fw>0&&this.query){this.overlayText(fx,fy,fw,'Filter: ',3);this.overlayText(fx+8,fy,fw-8,this.query);}
      const title= this.pack.rows[frame.rows[fy]]?.map(run=>run[1]).join('')||'';
      const count=title.match(/(?:PROCESSES|CONTAINERS|SERVICES|CONNECTIONS) (\d+)/);
      if(count){const at=title.indexOf(count[0])+count[0].lastIndexOf(count[1]);this.overlayText(at,fy,count[1].length,String(rows.length),5,1,true);}
      let headers=frame.sort_headers||[];
      if(!headers.length&&['network','containers','services'].includes(this.view)) {
        const hy=frame.table[1]-1,line=this.pack.rows[frame.rows[hy]]?.map(run=>run[1]).join('')||'';
        const fields=this.view==='network'?['TOTAL/s','PROCESS','STATE','DESTINATION','PID','PROTO','LOCAL','DOWN','UP','RECEIVED','SENT','RTT ms']:['NAME','UNIT','STATE','ACTIVE','CPU%','CPU','MEMORY'];
        const positions=fields.map(key=>({key,x:line.indexOf(key)})).filter(item=>item.x>=0).sort((a,b)=>a.x-b.x);
        const columns=positions.filter((item,i)=>!positions.slice(0,i).some(other=>other.x===item.x));
        headers=columns.map((item,i)=>[[item.x,hy,Math.max(1,(columns[i+1]?.x??(x+width))-item.x-1),1],({'UNIT':'NAME','CPU%':'CPU','MEMORY':'RAM','ACTIVE':'STATE'})[item.key]||item.key]);
      }
      for(const [rect,key] of headers){
        const [hx,hy,hw,hh]=rect;
        const header=this.pack.rows[frame.rows[hy]].map(run=>run[1]).join('').slice(hx,hx+hw).replace(/[↓↑]/g,'').trimEnd();
        const active=(this.sortKey||(this.view==='network'?'TOTAL/s':'CPU'))===key,ascending=['NAME','PID','USER','STATE','PROCESS','DESTINATION','PROTO','LOCAL','LPORT','DPORT'].includes(key)!==this.sortReverse;
        if(this.sortKey&&['processes','network'].includes(this.view))this.overlayText(hx,hy,hw,header+(active?(ascending?' ↑':' ↓'):''),this.view==='network'&&active?5:3,1,true);
        const hit=node('g',{'data-native-sort':key,role:'button',tabindex:0,'aria-label':`Sort by ${key}`});
        hit.append(node('rect',{x:16+hx*8,y:16+hy*18,width:hw*8,height:Math.max(1,hh)*18,fill:'transparent',class:'native-hit'}));this.hits.append(hit);
      }
    }
    draw() {
      if (!this.pack) return;
      const state=this.scenario==='live'?this.tableState():null;
      const frame=state?state.frame:this.pack.sequences[this.scenario][this.frame];
      frame.rows.forEach((id,y)=> {if (this.lastRows[y]!==id) this.groups[y].replaceChildren(this.renderRow(id));});
      this.lastRows=frame.rows;
      if(state)this.composeTable(frame,state.base,state.rows);else{this.overlays.replaceChildren();this.hits.replaceChildren();}
      this.setAttribute('data-frame',String(this.frame));
      this.setAttribute('data-scenario',this.scenario);
      this.updateControls();
    }
    updateControls() {
      this.querySelector('.native-table-tools').hidden=!['processes','network','containers','services'].includes(this.view);
      this.querySelector('[data-native-action="tree"]').hidden=this.view!=='processes';
      this.querySelector('[data-native-action="tree"]').setAttribute('aria-pressed',String(this.tree));
      this.querySelector('[data-native-action="close-details"]').hidden=!this.inspectedId;
    }
  }
  customElements.define('rstop-native',NativeDemo);
})();
