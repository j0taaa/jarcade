const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const vm=require('node:vm');
function adapter({mobile=true,url='https://example.test/',history:existing}={}) {
  let plugin,wakes=0,handler;
  const location=new URL(url), entries=existing || [{state:null,url:location.href}];
  let index=entries.length-1;
  const memory={buffer:new ArrayBuffer(2048)};
  const history={
    get state(){return entries[index].state;},
    replaceState(state,_,url){entries[index]={state,url:String(url)};location.href=String(url);},
    pushState(state,_,url){entries.splice(index+1);entries.push({state,url:String(url)});index++;location.href=String(url);},
    back(){if(index>0){index--;location.href=entries[index].url;handler({state:entries[index].state});}else{this.left=true;}},
    forward(){if(index<entries.length-1){index++;location.href=entries[index].url;handler({state:entries[index].state});}},
  };
  const context={TextEncoder,TextDecoder,Uint8Array,URL,URLSearchParams,location,history,
    window:{matchMedia:()=>({matches:mobile}),addEventListener:(name,fn)=>{if(name==='popstate')handler=fn;}},
    document:{hidden:false}, wasm_memory:memory, wasm_exports:{jarcade_wake(){wakes++;}},
    miniquad_add_plugin(value){plugin=value;}};
  vm.runInNewContext(fs.readFileSync('web/navigation.js','utf8'),context);
  const imports={env:{}};plugin.register_plugin(imports);plugin.on_init();const env=imports.env;
  const read=fn=>{const n=fn(0,2048);return new TextDecoder().decode(new Uint8Array(memory.buffer,0,n));};
  const sync=(path,replace=false)=>{const bytes=new TextEncoder().encode(path);new Uint8Array(memory.buffer).set(bytes);env.jarcade_route_sync(0,bytes.length,Number(replace));};
  return {env,history,entries,location,sync,load:()=>read(env.jarcade_route_load),poll:()=>{const v=read(env.jarcade_route_poll);return v?JSON.parse(v):null;},wakes:()=>wakes};
}
test('mobile Back opens navigation on the current route; the next Back can leave',()=>{
  const app=adapter({url:'https://example.test/games/sudoku/play'});
  app.sync(app.load(),true);app.history.back();
  assert.deepEqual(app.poll(),{route:'/games/sudoku/play',drawer:true});
  assert.equal(app.location.pathname,'/games/sudoku/play');
  app.env.jarcade_route_drawer(0);assert.deepEqual(app.poll(),{route:'/games/sudoku/play',drawer:false});
  app.history.back();app.poll();app.history.back();assert.equal(app.history.left,true);
  assert.equal(app.entries.length,2,'Back never inserts another guard');
});
test('mobile back/forward restores routes and sidebar states without growing history',()=>{
  const app=adapter();app.sync('/',true);app.sync('/multiplayer');app.sync('/games/wavelength');
  assert.equal(app.entries.length,6);
  app.history.back();assert.equal(app.poll().drawer,true);
  app.history.back();assert.deepEqual(app.poll(),{route:'/multiplayer',drawer:false});
  app.history.forward();assert.deepEqual(app.poll(),{route:'/games/wavelength',drawer:true});
  app.history.forward();assert.deepEqual(app.poll(),{route:'/games/wavelength',drawer:false});
  app.sync('/games/wavelength');assert.equal(app.entries.length,6);
});
test('desktop history goes directly to the previous route; menu opening is event-driven',()=>{
  const app=adapter({mobile:false});app.sync('/',true);app.sync('/settings');app.history.back();
  assert.deepEqual(app.poll(),{route:'/',drawer:false});
  app.env.jarcade_route_drawer(1);assert.deepEqual(app.poll(),{route:'/',drawer:true});
  app.env.jarcade_route_drawer(0);assert.equal(app.poll().drawer,false);
  assert.equal(app.entries.length,2);assert.equal(app.wakes(),3);
});
test('reload reuses the current mobile guard, and replacing a missing saved puzzle adds no guard',()=>{
  const app=adapter();app.sync('/',true);app.sync('/games/sudoku/play');
  const reload=adapter({url:app.location.href,history:app.entries});reload.sync(reload.load(),true);
  assert.equal(reload.entries.length,4);reload.sync('/games/sudoku',true);
  assert.equal(reload.entries.length,4);assert.equal(reload.location.pathname,'/games/sudoku');
});
test('legacy invites canonicalize paths, preserve the public code, and drop it on unrelated routes',()=>{
  const app=adapter({url:'https://example.test/?game=reverie&room=ABC123'});
  assert.equal(app.load(),'/games/dicksit');app.sync(app.load(),true);
  assert.equal(app.location.search,'?room=ABC123');app.sync('/multiplayer');assert.equal(app.location.search,'');
  const bad=adapter({url:'https://example.test/?game=__proto__'});assert.equal(bad.load(),'/');
});
test('rapid gestures deliver only the final state without timers or frame polling',()=>{
  const app=adapter();app.sync('/',true);app.sync('/games/nonograms');
  app.history.back();app.history.back();assert.deepEqual(app.poll(),{route:'/',drawer:false});assert.equal(app.poll(),null);
});
