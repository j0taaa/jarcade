// Canvas QA hit points for the responsive puzzle selector.
exports.setup=(w,h)=>{
  const landscape=w>=540&&h<500;
  const width=landscape?180:Math.min(w-32,840),x=landscape?16:(w-width)/2;
  return {
    size:i=>[x+(i+.5)*width/3,landscape?146:144],
    mode:i=>[x+(i+.5)*width/2,landscape?88:84],
    play:[landscape?106:w/2,h-40],
  };
};
