const test=require('node:test');const assert=require('node:assert/strict');const vm=require('node:vm');const fs=require('node:fs');
function setup(){let plugin,wakes=0;const storage=new Map(),sockets=[];const memory={buffer:new ArrayBuffer(200000)};class Socket{static OPEN=1;constructor(url){this.url=url;this.readyState=1;this.sent=[];sockets.push(this);}send(text){this.sent.push(text);}close(){this.closed=true;}}
 const ctx={miniquad_add_plugin:p=>plugin=p,TextDecoder,TextEncoder,Uint8Array,wasm_memory:memory,wasm_exports:{jarcade_wake:()=>wakes++},WebSocket:Socket,location:{pathname:'/',protocol:'https:',host:'example.test',search:'?game=reverie&room=ABC123',href:'https://example.test/?game=reverie&room=ABC123'},URL,URLSearchParams,document:{hidden:false},navigator:{},localStorage:{getItem:k=>storage.get(k),setItem:(k,v)=>storage.set(k,v)}};vm.runInNewContext(fs.readFileSync('web/multiplayer.js','utf8'),ctx);const imports={env:{}};plugin.register_plugin(imports);plugin.on_init();const env=imports.env;
 const put=t=>{const data=new TextEncoder().encode(t);new Uint8Array(memory.buffer).set(data);return data.length;};const poll=fn=>{const n=fn(0,200000);return n?new TextDecoder().decode(new Uint8Array(memory.buffer,0,n)):null;};return{ctx,env,sockets,put,poll,wakes:()=>wakes};}
test('socket events wake only visible screens; old socket events cannot replace the new seat',()=>{const s=setup();s.env.jarcade_online_connect();assert.equal(s.sockets[0].url,'wss://example.test/ws');s.sockets[0].onopen();assert.deepEqual(JSON.parse(s.poll(s.env.jarcade_online_poll)),{type:'connected'});s.ctx.document.hidden=true;s.sockets[0].onmessage({data:'{"type":"left"}'});assert.equal(s.wakes(),1);assert.equal(JSON.parse(s.poll(s.env.jarcade_online_poll)).type,'left');s.env.jarcade_online_connect();s.sockets[0].onclose();assert.equal(s.poll(s.env.jarcade_online_poll),null);s.ctx.document.hidden=false;s.sockets[1].onopen();assert.equal(s.wakes(),2);});
test('wire sends and bounded FIFO preserve private state without polling timers',()=>{const s=setup();s.env.jarcade_online_connect();const text='{"type":"join","room":"ABC123","name":"Mia"}';s.env.jarcade_online_send(0,s.put(text));assert.equal(s.sockets[0].sent[0],text);for(let i=0;i<150;i++)s.sockets[0].onmessage({data:JSON.stringify({type:'error',message:String(i)})});assert.equal(JSON.parse(s.poll(s.env.jarcade_online_poll)).message,'22');s.sockets[0].onmessage({data:'broken'});s.env.jarcade_online_close();assert.equal(s.poll(s.env.jarcade_online_poll),null);});
test('sessions are local, invite only contains public room code, and unavailable clipboard is safe',()=>{const s=setup();const value='{"sessions":[{"token":"private","room":"ABC123"}]}';assert.equal(s.env.jarcade_session_save(0,s.put(value)),1);assert.equal(s.poll(s.env.jarcade_session_load),value);assert.deepEqual(JSON.parse(s.poll(s.env.jarcade_invite_load)),{game:'reverie',room:'ABC123'});s.env.jarcade_copy_invite(0,s.put('{"game":"reverie","room":"ABC123"}'));});

test('text editors select a Wavelength side, commit on blur/Enter, and report Escape cancellation', () => {
 const s=setup(); const inputs=[]; const canvas={focus(){},getBoundingClientRect(){return{left:4,top:8};}};
 s.ctx.document={hidden:false,body:{style:{backgroundColor:'#f5ebd9'},appendChild(input){inputs.push(input);}},getElementById(){return canvas;},createElement(){return{style:{},attrs:{},setAttribute(k,v){this.attrs[k]=v;},focus(){},remove(){this.removed=true;},setSelectionRange(a,b){this.selection=[a,b];},select(){this.selection=[0,this.value.length];}};}};
 const open=(id,value)=>{s.env.jarcade_editor_open(0,s.put(value),id,16,40,100,52,120);return inputs.at(-1);};
 let input=open(3,'A chilly morning');
 assert.equal(input.attrs['aria-label'],'First extreme');
 assert.deepEqual(input.selection,[0,input.value.length]);
 assert.equal(input.style.left,'20px'); assert.equal(input.maxLength,120);
 s.env.jarcade_editor_position(4,80,90,110,52); assert.equal(input.style.left,'20px');
 s.env.jarcade_editor_position(3,80,90,110,52); assert.equal(input.style.left,'84px'); assert.equal(input.style.top,'98px');
 assert.equal(input.style.width,'110px'); assert.deepEqual(input.selection,[0,input.value.length]);
 input.value='Frio de inverno'; input.oninput();
 assert.equal(JSON.parse(s.poll(s.env.jarcade_editor_poll)).done,false);
 input.onblur();
 assert.deepEqual(JSON.parse(s.poll(s.env.jarcade_editor_poll)),{id:3,text:'Frio de inverno',done:true});
 assert.equal(input.removed,true);
 input=open(4,'Hot'); input.value='Canceled';
 input.onkeydown({key:'Escape',stopPropagation(){},preventDefault(){}});
 assert.deepEqual(JSON.parse(s.poll(s.env.jarcade_editor_poll)),{id:4,text:'Canceled',done:true,cancelled:true});
 input.onblur(); assert.equal(s.poll(s.env.jarcade_editor_poll),null);
 input=open(2,'A story clue');
 assert.deepEqual(input.selection,[input.value.length,input.value.length]);
 input.onkeydown({key:'Enter',stopPropagation(){},preventDefault(){}});
 assert.deepEqual(JSON.parse(s.poll(s.env.jarcade_editor_poll)),{id:2,text:'A story clue',done:true,cancelled:false});
});

test('clean game routes load public room invites and copied links contain no private session data',async()=>{
 const s=setup();s.ctx.location.pathname='/games/coupe';s.ctx.location.search='?room=ABC123';
 assert.deepEqual(JSON.parse(s.poll(s.env.jarcade_invite_load)),{game:'court',room:'ABC123'});
 let copied;s.ctx.navigator.clipboard={writeText:async value=>{copied=value;}};
 s.env.jarcade_copy_invite(0,s.put('{"game":"court","room":"ABC123"}'));
 const link=new URL(copied);assert.equal(link.pathname,'/games/coupe');assert.equal(link.search,'?room=ABC123');
 assert(!copied.includes('token'));assert(!copied.includes('game='));
});


test('Drawing Telephone route invites contain only the room code and vector drafts retain UTF-8 data',async()=>{
 const s=setup();s.ctx.location.pathname='/games/drawing-telephone';s.ctx.location.search='?room=ABC123';
 assert.deepEqual(JSON.parse(s.poll(s.env.jarcade_invite_load)),{game:'telephone',room:'ABC123'});
 let copied;s.ctx.navigator.clipboard={writeText:async value=>{copied=value;}};
 s.env.jarcade_copy_invite(0,s.put('{"game":"telephone","room":"ABC123"}'));
 const link=new URL(copied);assert.equal(link.pathname,'/games/drawing-telephone');assert.equal(link.search,'?room=ABC123');
 assert(!copied.includes('token'));assert(!copied.includes('game='));
 const draft=JSON.stringify({sessions:[{game:'telephone',room:'ABC123',token:'private'}],telephone_draft:{key:'ABC123:0:1',text:'Um peixe na lua 🐟',drawing:{strokes:[{color:1,width:6,points:Array(4096).fill([1000,750])}]}}});
 assert(draft.length>4096);assert.equal(s.env.jarcade_session_save(0,s.put(draft)),1);
 assert.equal(s.poll(s.env.jarcade_session_load),draft);
});
