/* Platform transport and phone IME only; rules and interface are shared Rust. */
(() => {
  "use strict";
  let socket = null, generation = 0, ready = false, editor = null;
  const queue = [], edits = [];
  const decode = (p,n) => new TextDecoder().decode(new Uint8Array(wasm_memory.buffer,p,n));
  const wake = () => { if (ready && !document.hidden) wasm_exports.jarcade_wake(); };
  const emit = data => { if (queue.length >= 128) queue.shift(); queue.push(JSON.stringify(data)); wake(); };
  function close() { generation++; if (socket) socket.close(); socket=null; queue.length=0; }
  function hideEditor() { if(editor) { const old=editor; editor=null; old.remove(); document.getElementById("glcanvas").focus({preventScroll:true}); } }
  function copy(p,capacity,text) { const bytes=new TextEncoder().encode(text); const n=Math.min(capacity,bytes.length); new Uint8Array(wasm_memory.buffer,p,n).set(bytes.subarray(0,n)); return n; }
  miniquad_add_plugin({ name:"jarcade_multiplayer",version:1,
    register_plugin(imports) {
      const env=imports.env;
      env.jarcade_online_connect=()=>{
        close(); const ticket=generation; const protocol=location.protocol==="https:"?"wss:":"ws:";
        try { socket=new WebSocket(`${protocol}//${location.host}/ws`); } catch { emit({type:"disconnected",reason:"Could not connect. Retry."}); return; }
        socket.onopen=()=>{if(ticket===generation)emit({type:"connected"});};
        socket.onmessage=event=>{if(ticket!==generation||typeof event.data!=="string"||event.data.length>131072)return;try{emit(JSON.parse(event.data));}catch{emit({type:"error",message:"Invalid server response"});}};
        socket.onclose=()=>{if(ticket===generation){socket=null;emit({type:"disconnected",reason:"Connection lost. Reconnect to keep playing."});}};
        socket.onerror=()=>{};
      };
      env.jarcade_online_close=close;
      env.jarcade_online_send=(p,n)=>{if(socket?.readyState===WebSocket.OPEN)socket.send(decode(p,n));};
      env.jarcade_online_poll=(p,n)=>queue.length?copy(p,n,queue.shift()):0;
      env.jarcade_session_load=(p,n)=>{try{return copy(p,n,localStorage.getItem("jarcade.online.v1")||"");}catch{return 0;}};
      env.jarcade_session_save=(p,n)=>{try{localStorage.setItem("jarcade.online.v1",decode(p,n));return 1;}catch{return 0;}};
      env.jarcade_invite_load=(p,n)=>copy(p,n,JSON.stringify({game:({coupe:"court",dicksit:"reverie",wolvesville:"wolves",codenames:"codenames"})[location.pathname.split("/")[2]]||new URLSearchParams(location.search).get("game"),room:new URLSearchParams(location.search).get("room")}));
      env.jarcade_copy_invite=(p,n)=>{const data=JSON.parse(decode(p,n));const url=new URL(location.href);url.pathname="/games/"+({court:"coupe",reverie:"dicksit",wolves:"wolvesville",codenames:"codenames"})[data.game];url.hash="";url.search="";url.searchParams.set("room",data.room);navigator.clipboard?.writeText(url.href).catch(()=>{});};
      env.jarcade_editor_open=(p,n,id,x,y,w,h,max)=>{
        const value=decode(p,n); const bounds=document.getElementById("glcanvas").getBoundingClientRect(); x+=bounds.left; y+=bounds.top; hideEditor(); const input=document.createElement("input");editor=input;
        input.id="jarcade-text-editor";input.jarcadeField=id;input.value=value;input.maxLength=max;input.autocomplete="off";input.spellcheck=id>=2;input.enterKeyHint="done";input.setAttribute("aria-label",["Player name","Room code","Story clue","First extreme","Second extreme","Message","One-word clue"][id]);
        Object.assign(input.style,{position:"fixed",left:`${x}px`,top:`${y}px`,width:`${w}px`,height:`${h}px`,border:`2px solid ${id===6?"#2f74bb":id===5?"#515387":id>=3?"#e2644e":"#1b6d4b"}`,borderRadius:"14px",background:document.body.style.backgroundColor||"white",color:document.body.style.backgroundColor==="rgb(0, 0, 0)"?"white":"#18221c",font:"16px system-ui",padding:"0 14px",zIndex:10,outline:"none"});
        input.oninput=()=>{edits.push(JSON.stringify({id,text:input.value,done:false}));wake();};
        input.onkeydown=event=>{event.stopPropagation();if(event.key==="Enter"||event.key==="Escape"){event.preventDefault();edits.push(JSON.stringify({id,text:input.value,done:true,cancelled:event.key==="Escape"}));hideEditor();wake();}};
        input.onblur=()=>{if(editor===input){edits.push(JSON.stringify({id,text:input.value,done:true}));hideEditor();wake();}};
        document.body.appendChild(input);input.focus({preventScroll:true});if(id===3||id===4)input.select();else input.setSelectionRange(input.value.length,input.value.length);
      };
      env.jarcade_editor_position=(id,x,y,w,h)=>{
        if(!editor || editor.jarcadeField!==id)return;
        const bounds=document.getElementById("glcanvas").getBoundingClientRect();
        Object.assign(editor.style,{left:`${x+bounds.left}px`,top:`${y+bounds.top}px`,width:`${w}px`,height:`${h}px`});
      };
      env.jarcade_editor_close=()=>{hideEditor();edits.length=0;};
      env.jarcade_editor_poll=(p,n)=>edits.length?copy(p,n,edits.shift()):0;
    },on_init(){ready=true;},
  });
})();
