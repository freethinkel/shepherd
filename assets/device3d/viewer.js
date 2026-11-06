(()=>{function yo(s){let e=s.hingeAngle??(s.screenId===1?0:180);return s.screenId===1==(e===0)}function eu(s){let e=null;return{begin(t,n){return s.contains(t,n)?(e=Math.max(0,Math.min(180,s.angle())),!0):!1},move(t){if(e===null||!Number.isFinite(t))return;let n=Math.max(0,Math.min(180,e+t*120));n!==e&&(e=n,s.change(n))},end(){e!==null&&(e=null,s.change(null))},get active(){return e!==null}}}var Vu=0,bc=1,Gu=2;var Gr=1,Hu=2,Gs=3,Ri=0,Xt=1,Gn=2,$i=0,Hs=1,wc=2,Ac=3,Tc=4,Wu=5;var Ki=100,Xu=101,qu=102,Yu=103,Zu=104,Ju=200,$u=201,Ku=202,ju=203,Ec=204,Rc=205,Qu=206,ef=207,tf=208,nf=209,sf=210,rf=211,of=212,af=213,lf=214,Xo=0,qo=1,Yo=2,bs=3,Zo=4,Jo=5,$o=6,Ko=7,va=0,cf=1,hf=2,Hn=0,Cc=1,Pc=2,Ic=3,Dc=4,Lc=5,Nc=6,Uc=7,ac="attached",uf="detached",Fc=300,ji=301,Ws=302,Sa=303,Ma=304,Gi=1e3,_n=1001,ws=1002,Vt=1003,ff=1004;var Hr=1005;var Dt=1006,ba=1007;var fi=1008;var on=1009,Oc=1010,Bc=1011,Xs=1012,wa=1013,Pn=1014,dn=1015,In=1016,Aa=1017,Ta=1018,Qi=1020,zc=35902,kc=35899,Vc=1021,Gc=1022,Mn=1023,ii=1026,di=1027,Ea=1028,Ra=1029,Ci=1030,Ca=1031;var Pa=1033,Wr=33776,Xr=33777,qr=33778,Yr=33779,Ia=35840,Da=35841,La=35842,Na=35843,Ua=36196,Fa=37492,Oa=37496,Ba=37488,za=37489,Zr=37490,ka=37491,Va=37808,Ga=37809,Ha=37810,Wa=37811,Xa=37812,qa=37813,Ya=37814,Za=37815,Ja=37816,$a=37817,Ka=37818,ja=37819,Qa=37820,el=37821,tl=36492,nl=36494,il=36495,sl=36283,rl=36284,Jr=36285,ol=36286;var dr=2300,jo=2301,Ho=2302,lc=2303,cc=2400,hc=2401,uc=2402,df=2500;var pf=3200;var $r=0,mf=1,qt="",wt="srgb",Gt="srgb-linear",pr="linear",pt="srgb";var Wo=7680;var gf=519,_f=512,xf=513,yf=514,Hc=515,vf=516,Sf=517,Wc=518,Xc=519,Mf=35044;var qc="300 es",xn=2e3,As=2001;function pp(s){for(let e=s.length-1;e>=0;--e)if(s[e]>=65535)return!0;return!1}function mp(s){return ArrayBuffer.isView(s)&&!(s instanceof DataView)}function Ts(s){return document.createElementNS("http://www.w3.org/1999/xhtml",s)}function bf(){let s=Ts("canvas");return s.style.display="block",s}var tu={},Es=null;function Yc(...s){let e="THREE."+s.shift();Es?Es("log",e,...s):console.log(e,...s)}function wf(s){let e=s[0];if(typeof e=="string"&&e.startsWith("TSL:")){let t=s[1];t&&t.isStackTrace?s[0]+=" "+t.getLocation():s[1]='Stack trace not available. Enable "THREE.Node.captureStackTrace" to capture stack traces.'}return s}function Xe(...s){s=wf(s);let e="THREE."+s.shift();if(Es)Es("warn",e,...s);else{let t=s[0];t&&t.isStackTrace?console.warn(t.getError(e)):console.warn(e,...s)}}function Ye(...s){s=wf(s);let e="THREE."+s.shift();if(Es)Es("error",e,...s);else{let t=s[0];t&&t.isStackTrace?console.error(t.getError(e)):console.error(e,...s)}}function Vi(...s){let e=s.join(" ");e in tu||(tu[e]=!0,Xe(...s))}function Af(s,e,t){return new Promise(function(n,i){function r(){switch(s.clientWaitSync(e,s.SYNC_FLUSH_COMMANDS_BIT,0)){case s.WAIT_FAILED:i();break;case s.TIMEOUT_EXPIRED:setTimeout(r,t);break;default:n()}}setTimeout(r,t)})}var Tf={[Xo]:Xo,[Yo]:$o,[Zo]:Zo,[bs]:Jo,[qo]:qo,[$o]:Yo,[Ko]:Ko,[Jo]:bs},Bn=class{addEventListener(e,t){this._listeners===void 0&&(this._listeners={});let n=this._listeners;n[e]===void 0&&(n[e]=[]),n[e].indexOf(t)===-1&&n[e].push(t)}hasEventListener(e,t){let n=this._listeners;return n===void 0?!1:n[e]!==void 0&&n[e].indexOf(t)!==-1}removeEventListener(e,t){let n=this._listeners;if(n===void 0)return;let i=n[e];if(i!==void 0){let r=i.indexOf(t);r!==-1&&i.splice(r,1)}}dispatchEvent(e){let t=this._listeners;if(t===void 0)return;let n=t[e.type];if(n!==void 0){e.target=this;let i=n.slice(0);for(let r=0,o=i.length;r<o;r++)i[r].call(this,e);e.target=null}}},$t=["00","01","02","03","04","05","06","07","08","09","0a","0b","0c","0d","0e","0f","10","11","12","13","14","15","16","17","18","19","1a","1b","1c","1d","1e","1f","20","21","22","23","24","25","26","27","28","29","2a","2b","2c","2d","2e","2f","30","31","32","33","34","35","36","37","38","39","3a","3b","3c","3d","3e","3f","40","41","42","43","44","45","46","47","48","49","4a","4b","4c","4d","4e","4f","50","51","52","53","54","55","56","57","58","59","5a","5b","5c","5d","5e","5f","60","61","62","63","64","65","66","67","68","69","6a","6b","6c","6d","6e","6f","70","71","72","73","74","75","76","77","78","79","7a","7b","7c","7d","7e","7f","80","81","82","83","84","85","86","87","88","89","8a","8b","8c","8d","8e","8f","90","91","92","93","94","95","96","97","98","99","9a","9b","9c","9d","9e","9f","a0","a1","a2","a3","a4","a5","a6","a7","a8","a9","aa","ab","ac","ad","ae","af","b0","b1","b2","b3","b4","b5","b6","b7","b8","b9","ba","bb","bc","bd","be","bf","c0","c1","c2","c3","c4","c5","c6","c7","c8","c9","ca","cb","cc","cd","ce","cf","d0","d1","d2","d3","d4","d5","d6","d7","d8","d9","da","db","dc","dd","de","df","e0","e1","e2","e3","e4","e5","e6","e7","e8","e9","ea","eb","ec","ed","ee","ef","f0","f1","f2","f3","f4","f5","f6","f7","f8","f9","fa","fb","fc","fd","fe","ff"];var Nl=Math.PI/180,mr=180/Math.PI;function Pi(){let s=Math.random()*4294967295|0,e=Math.random()*4294967295|0,t=Math.random()*4294967295|0,n=Math.random()*4294967295|0;return($t[s&255]+$t[s>>8&255]+$t[s>>16&255]+$t[s>>24&255]+"-"+$t[e&255]+$t[e>>8&255]+"-"+$t[e>>16&15|64]+$t[e>>24&255]+"-"+$t[t&63|128]+$t[t>>8&255]+"-"+$t[t>>16&255]+$t[t>>24&255]+$t[n&255]+$t[n>>8&255]+$t[n>>16&255]+$t[n>>24&255]).toLowerCase()}function it(s,e,t){return Math.max(e,Math.min(t,s))}function gp(s,e){return(s%e+e)%e}function Ul(s,e,t){return(1-t)*s+t*e}function Ef(s){return Math.pow(2,Math.floor(Math.log(s)/Math.LN2))}function Zc(s){switch(s.constructor){case Uint32Array:return 4294967295;case Uint16Array:return 65535;case Uint8Array:case Uint8ClampedArray:return 255;case Int32Array:return 2147483647;case Int16Array:return 32767;case Int8Array:return 127;default:return}}function _p(s,e){if(e.constructor===Float32Array)return s;let t=Zc(e);if(t===void 0)throw new Error("THREE.MathUtils: Invalid component type.");return Math.max(s/t,-1)}function xp(s,e){if(e.constructor===Float32Array)return s;let t=Zc(e);if(t===void 0)throw new Error("THREE.MathUtils: Invalid component type.");return Math.round(s*t)}var eh=class eh{constructor(e=0,t=0){this.x=e,this.y=t}get width(){return this.x}set width(e){this.x=e}get height(){return this.y}set height(e){this.y=e}set(e,t){return this.x=e,this.y=t,this}setScalar(e){return this.x=e,this.y=e,this}setX(e){return this.x=e,this}setY(e){return this.y=e,this}setComponent(e,t){switch(e){case 0:this.x=t;break;case 1:this.y=t;break;default:throw new Error("THREE.Vector2: index is out of range: "+e)}return this}getComponent(e){switch(e){case 0:return this.x;case 1:return this.y;default:throw new Error("THREE.Vector2: index is out of range: "+e)}}clone(){return new this.constructor(this.x,this.y)}copy(e){return this.x=e.x,this.y=e.y,this}add(e){return this.x+=e.x,this.y+=e.y,this}addScalar(e){return this.x+=e,this.y+=e,this}addVectors(e,t){return this.x=e.x+t.x,this.y=e.y+t.y,this}addScaledVector(e,t){return this.x+=e.x*t,this.y+=e.y*t,this}sub(e){return this.x-=e.x,this.y-=e.y,this}subScalar(e){return this.x-=e,this.y-=e,this}subVectors(e,t){return this.x=e.x-t.x,this.y=e.y-t.y,this}multiply(e){return this.x*=e.x,this.y*=e.y,this}multiplyScalar(e){return this.x*=e,this.y*=e,this}divide(e){return this.x/=e.x,this.y/=e.y,this}divideScalar(e){return this.multiplyScalar(1/e)}applyMatrix3(e){let t=this.x,n=this.y,i=e.elements;return this.x=i[0]*t+i[3]*n+i[6],this.y=i[1]*t+i[4]*n+i[7],this}min(e){return this.x=Math.min(this.x,e.x),this.y=Math.min(this.y,e.y),this}max(e){return this.x=Math.max(this.x,e.x),this.y=Math.max(this.y,e.y),this}clamp(e,t){return this.x=it(this.x,e.x,t.x),this.y=it(this.y,e.y,t.y),this}clampScalar(e,t){return this.x=it(this.x,e,t),this.y=it(this.y,e,t),this}clampLength(e,t){let n=this.length();return this.divideScalar(n||1).multiplyScalar(it(n,e,t))}floor(){return this.x=Math.floor(this.x),this.y=Math.floor(this.y),this}ceil(){return this.x=Math.ceil(this.x),this.y=Math.ceil(this.y),this}round(){return this.x=Math.round(this.x),this.y=Math.round(this.y),this}roundToZero(){return this.x=Math.trunc(this.x),this.y=Math.trunc(this.y),this}negate(){return this.x=-this.x,this.y=-this.y,this}dot(e){return this.x*e.x+this.y*e.y}cross(e){return this.x*e.y-this.y*e.x}lengthSq(){return this.x*this.x+this.y*this.y}length(){return Math.sqrt(this.x*this.x+this.y*this.y)}manhattanLength(){return Math.abs(this.x)+Math.abs(this.y)}normalize(){return this.divideScalar(this.length()||1)}angle(){return Math.atan2(-this.y,-this.x)+Math.PI}angleTo(e){let t=Math.sqrt(this.lengthSq()*e.lengthSq());if(t===0)return Math.PI/2;let n=this.dot(e)/t;return Math.acos(it(n,-1,1))}distanceTo(e){return Math.sqrt(this.distanceToSquared(e))}distanceToSquared(e){let t=this.x-e.x,n=this.y-e.y;return t*t+n*n}manhattanDistanceTo(e){return Math.abs(this.x-e.x)+Math.abs(this.y-e.y)}setLength(e){return this.normalize().multiplyScalar(e)}lerp(e,t){return this.x+=(e.x-this.x)*t,this.y+=(e.y-this.y)*t,this}lerpVectors(e,t,n){return this.x=e.x+(t.x-e.x)*n,this.y=e.y+(t.y-e.y)*n,this}equals(e){return e.x===this.x&&e.y===this.y}fromArray(e,t=0){return this.x=e[t],this.y=e[t+1],this}toArray(e=[],t=0){return e[t]=this.x,e[t+1]=this.y,e}fromBufferAttribute(e,t){return this.x=e.getX(t),this.y=e.getY(t),this}rotateAround(e,t){let n=Math.cos(t),i=Math.sin(t),r=this.x-e.x,o=this.y-e.y;return this.x=r*n-o*i+e.x,this.y=r*i+o*n+e.y,this}random(){return this.x=Math.random(),this.y=Math.random(),this}*[Symbol.iterator](){yield this.x,yield this.y}};eh.prototype.isVector2=!0;var pe=eh,st=class{constructor(e=0,t=0,n=0,i=1){this.isQuaternion=!0,this._x=e,this._y=t,this._z=n,this._w=i}static slerpFlat(e,t,n,i,r,o,a){let l=n[i+0],c=n[i+1],u=n[i+2],h=n[i+3],d=r[o+0],f=r[o+1],m=r[o+2],_=r[o+3];if(h!==_||l!==d||c!==f||u!==m){let p=l*d+c*f+u*m+h*_;p<0&&(d=-d,f=-f,m=-m,_=-_,p=-p);let g=1-a;if(p<.9995){let S=Math.acos(p),v=Math.sin(S);g=Math.sin(g*S)/v,a=Math.sin(a*S)/v,l=l*g+d*a,c=c*g+f*a,u=u*g+m*a,h=h*g+_*a}else{l=l*g+d*a,c=c*g+f*a,u=u*g+m*a,h=h*g+_*a;let S=1/Math.sqrt(l*l+c*c+u*u+h*h);l*=S,c*=S,u*=S,h*=S}}e[t]=l,e[t+1]=c,e[t+2]=u,e[t+3]=h}static multiplyQuaternionsFlat(e,t,n,i,r,o){let a=n[i],l=n[i+1],c=n[i+2],u=n[i+3],h=r[o],d=r[o+1],f=r[o+2],m=r[o+3];return e[t]=a*m+u*h+l*f-c*d,e[t+1]=l*m+u*d+c*h-a*f,e[t+2]=c*m+u*f+a*d-l*h,e[t+3]=u*m-a*h-l*d-c*f,e}get x(){return this._x}set x(e){this._x=e,this._onChangeCallback()}get y(){return this._y}set y(e){this._y=e,this._onChangeCallback()}get z(){return this._z}set z(e){this._z=e,this._onChangeCallback()}get w(){return this._w}set w(e){this._w=e,this._onChangeCallback()}set(e,t,n,i){return this._x=e,this._y=t,this._z=n,this._w=i,this._onChangeCallback(),this}clone(){return new this.constructor(this._x,this._y,this._z,this._w)}copy(e){return this._x=e.x,this._y=e.y,this._z=e.z,this._w=e.w,this._onChangeCallback(),this}setFromEuler(e,t=!0){let n=e._x,i=e._y,r=e._z,o=e._order,a=Math.cos,l=Math.sin,c=a(n/2),u=a(i/2),h=a(r/2),d=l(n/2),f=l(i/2),m=l(r/2);switch(o){case"XYZ":this._x=d*u*h+c*f*m,this._y=c*f*h-d*u*m,this._z=c*u*m+d*f*h,this._w=c*u*h-d*f*m;break;case"YXZ":this._x=d*u*h+c*f*m,this._y=c*f*h-d*u*m,this._z=c*u*m-d*f*h,this._w=c*u*h+d*f*m;break;case"ZXY":this._x=d*u*h-c*f*m,this._y=c*f*h+d*u*m,this._z=c*u*m+d*f*h,this._w=c*u*h-d*f*m;break;case"ZYX":this._x=d*u*h-c*f*m,this._y=c*f*h+d*u*m,this._z=c*u*m-d*f*h,this._w=c*u*h+d*f*m;break;case"YZX":this._x=d*u*h+c*f*m,this._y=c*f*h+d*u*m,this._z=c*u*m-d*f*h,this._w=c*u*h-d*f*m;break;case"XZY":this._x=d*u*h-c*f*m,this._y=c*f*h-d*u*m,this._z=c*u*m+d*f*h,this._w=c*u*h+d*f*m;break;default:Xe("Quaternion: .setFromEuler() encountered an unknown order: "+o)}return t===!0&&this._onChangeCallback(),this}setFromAxisAngle(e,t){let n=t/2,i=Math.sin(n);return this._x=e.x*i,this._y=e.y*i,this._z=e.z*i,this._w=Math.cos(n),this._onChangeCallback(),this}setFromRotationMatrix(e){let t=e.elements,n=t[0],i=t[4],r=t[8],o=t[1],a=t[5],l=t[9],c=t[2],u=t[6],h=t[10],d=n+a+h;if(d>0){let f=.5/Math.sqrt(d+1);this._w=.25/f,this._x=(u-l)*f,this._y=(r-c)*f,this._z=(o-i)*f}else if(n>a&&n>h){let f=2*Math.sqrt(1+n-a-h);this._w=(u-l)/f,this._x=.25*f,this._y=(i+o)/f,this._z=(r+c)/f}else if(a>h){let f=2*Math.sqrt(1+a-n-h);this._w=(r-c)/f,this._x=(i+o)/f,this._y=.25*f,this._z=(l+u)/f}else{let f=2*Math.sqrt(1+h-n-a);this._w=(o-i)/f,this._x=(r+c)/f,this._y=(l+u)/f,this._z=.25*f}return this._onChangeCallback(),this}setFromUnitVectors(e,t){let n=e.dot(t)+1;return n<1e-8?(n=0,Math.abs(e.x)>Math.abs(e.z)?(this._x=-e.y,this._y=e.x,this._z=0,this._w=n):(this._x=0,this._y=-e.z,this._z=e.y,this._w=n)):(this._x=e.y*t.z-e.z*t.y,this._y=e.z*t.x-e.x*t.z,this._z=e.x*t.y-e.y*t.x,this._w=n),this.normalize()}angleTo(e){return 2*Math.acos(Math.abs(it(this.dot(e),-1,1)))}rotateTowards(e,t){let n=this.angleTo(e);if(n===0)return this;let i=Math.min(1,t/n);return this.slerp(e,i),this}identity(){return this.set(0,0,0,1)}invert(){return this.conjugate()}conjugate(){return this._x*=-1,this._y*=-1,this._z*=-1,this._onChangeCallback(),this}dot(e){return this._x*e._x+this._y*e._y+this._z*e._z+this._w*e._w}lengthSq(){return this._x*this._x+this._y*this._y+this._z*this._z+this._w*this._w}length(){return Math.sqrt(this._x*this._x+this._y*this._y+this._z*this._z+this._w*this._w)}normalize(){let e=this.length();return e===0?(this._x=0,this._y=0,this._z=0,this._w=1):(e=1/e,this._x=this._x*e,this._y=this._y*e,this._z=this._z*e,this._w=this._w*e),this._onChangeCallback(),this}multiply(e){return this.multiplyQuaternions(this,e)}premultiply(e){return this.multiplyQuaternions(e,this)}multiplyQuaternions(e,t){let n=e._x,i=e._y,r=e._z,o=e._w,a=t._x,l=t._y,c=t._z,u=t._w;return this._x=n*u+o*a+i*c-r*l,this._y=i*u+o*l+r*a-n*c,this._z=r*u+o*c+n*l-i*a,this._w=o*u-n*a-i*l-r*c,this._onChangeCallback(),this}slerp(e,t){let n=e._x,i=e._y,r=e._z,o=e._w,a=this.dot(e);a<0&&(n=-n,i=-i,r=-r,o=-o,a=-a);let l=1-t;if(a<.9995){let c=Math.acos(a),u=Math.sin(c);l=Math.sin(l*c)/u,t=Math.sin(t*c)/u,this._x=this._x*l+n*t,this._y=this._y*l+i*t,this._z=this._z*l+r*t,this._w=this._w*l+o*t,this._onChangeCallback()}else this._x=this._x*l+n*t,this._y=this._y*l+i*t,this._z=this._z*l+r*t,this._w=this._w*l+o*t,this.normalize();return this}slerpQuaternions(e,t,n){return this.copy(e).slerp(t,n)}random(){let e=2*Math.PI*Math.random(),t=2*Math.PI*Math.random(),n=Math.random(),i=Math.sqrt(1-n),r=Math.sqrt(n);return this.set(i*Math.sin(e),i*Math.cos(e),r*Math.sin(t),r*Math.cos(t))}equals(e){return e._x===this._x&&e._y===this._y&&e._z===this._z&&e._w===this._w}fromArray(e,t=0){return this._x=e[t],this._y=e[t+1],this._z=e[t+2],this._w=e[t+3],this._onChangeCallback(),this}toArray(e=[],t=0){return e[t]=this._x,e[t+1]=this._y,e[t+2]=this._z,e[t+3]=this._w,e}fromBufferAttribute(e,t){return this._x=e.getX(t),this._y=e.getY(t),this._z=e.getZ(t),this._w=e.getW(t),this._onChangeCallback(),this}toJSON(){return this.toArray()}_onChange(e){return this._onChangeCallback=e,this}_onChangeCallback(){}*[Symbol.iterator](){yield this._x,yield this._y,yield this._z,yield this._w}},th=class th{constructor(e=0,t=0,n=0){this.x=e,this.y=t,this.z=n}set(e,t,n){return n===void 0&&(n=this.z),this.x=e,this.y=t,this.z=n,this}setScalar(e){return this.x=e,this.y=e,this.z=e,this}setX(e){return this.x=e,this}setY(e){return this.y=e,this}setZ(e){return this.z=e,this}setComponent(e,t){switch(e){case 0:this.x=t;break;case 1:this.y=t;break;case 2:this.z=t;break;default:throw new Error("THREE.Vector3: index is out of range: "+e)}return this}getComponent(e){switch(e){case 0:return this.x;case 1:return this.y;case 2:return this.z;default:throw new Error("THREE.Vector3: index is out of range: "+e)}}clone(){return new this.constructor(this.x,this.y,this.z)}copy(e){return this.x=e.x,this.y=e.y,this.z=e.z,this}add(e){return this.x+=e.x,this.y+=e.y,this.z+=e.z,this}addScalar(e){return this.x+=e,this.y+=e,this.z+=e,this}addVectors(e,t){return this.x=e.x+t.x,this.y=e.y+t.y,this.z=e.z+t.z,this}addScaledVector(e,t){return this.x+=e.x*t,this.y+=e.y*t,this.z+=e.z*t,this}sub(e){return this.x-=e.x,this.y-=e.y,this.z-=e.z,this}subScalar(e){return this.x-=e,this.y-=e,this.z-=e,this}subVectors(e,t){return this.x=e.x-t.x,this.y=e.y-t.y,this.z=e.z-t.z,this}multiply(e){return this.x*=e.x,this.y*=e.y,this.z*=e.z,this}multiplyScalar(e){return this.x*=e,this.y*=e,this.z*=e,this}multiplyVectors(e,t){return this.x=e.x*t.x,this.y=e.y*t.y,this.z=e.z*t.z,this}applyEuler(e){return this.applyQuaternion(nu.setFromEuler(e))}applyAxisAngle(e,t){return this.applyQuaternion(nu.setFromAxisAngle(e,t))}applyMatrix3(e){let t=this.x,n=this.y,i=this.z,r=e.elements;return this.x=r[0]*t+r[3]*n+r[6]*i,this.y=r[1]*t+r[4]*n+r[7]*i,this.z=r[2]*t+r[5]*n+r[8]*i,this}applyNormalMatrix(e){return this.applyMatrix3(e).normalize()}applyMatrix4(e){let t=this.x,n=this.y,i=this.z,r=e.elements,o=1/(r[3]*t+r[7]*n+r[11]*i+r[15]);return this.x=(r[0]*t+r[4]*n+r[8]*i+r[12])*o,this.y=(r[1]*t+r[5]*n+r[9]*i+r[13])*o,this.z=(r[2]*t+r[6]*n+r[10]*i+r[14])*o,this}applyQuaternion(e){let t=this.x,n=this.y,i=this.z,r=e.x,o=e.y,a=e.z,l=e.w,c=2*(o*i-a*n),u=2*(a*t-r*i),h=2*(r*n-o*t);return this.x=t+l*c+o*h-a*u,this.y=n+l*u+a*c-r*h,this.z=i+l*h+r*u-o*c,this}project(e){return this.applyMatrix4(e.matrixWorldInverse).applyMatrix4(e.projectionMatrix)}unproject(e){return this.applyMatrix4(e.projectionMatrixInverse).applyMatrix4(e.matrixWorld)}transformDirection(e){let t=this.x,n=this.y,i=this.z,r=e.elements;return this.x=r[0]*t+r[4]*n+r[8]*i,this.y=r[1]*t+r[5]*n+r[9]*i,this.z=r[2]*t+r[6]*n+r[10]*i,this.normalize()}divide(e){return this.x/=e.x,this.y/=e.y,this.z/=e.z,this}divideScalar(e){return this.multiplyScalar(1/e)}min(e){return this.x=Math.min(this.x,e.x),this.y=Math.min(this.y,e.y),this.z=Math.min(this.z,e.z),this}max(e){return this.x=Math.max(this.x,e.x),this.y=Math.max(this.y,e.y),this.z=Math.max(this.z,e.z),this}clamp(e,t){return this.x=it(this.x,e.x,t.x),this.y=it(this.y,e.y,t.y),this.z=it(this.z,e.z,t.z),this}clampScalar(e,t){return this.x=it(this.x,e,t),this.y=it(this.y,e,t),this.z=it(this.z,e,t),this}clampLength(e,t){let n=this.length();return this.divideScalar(n||1).multiplyScalar(it(n,e,t))}floor(){return this.x=Math.floor(this.x),this.y=Math.floor(this.y),this.z=Math.floor(this.z),this}ceil(){return this.x=Math.ceil(this.x),this.y=Math.ceil(this.y),this.z=Math.ceil(this.z),this}round(){return this.x=Math.round(this.x),this.y=Math.round(this.y),this.z=Math.round(this.z),this}roundToZero(){return this.x=Math.trunc(this.x),this.y=Math.trunc(this.y),this.z=Math.trunc(this.z),this}negate(){return this.x=-this.x,this.y=-this.y,this.z=-this.z,this}dot(e){return this.x*e.x+this.y*e.y+this.z*e.z}lengthSq(){return this.x*this.x+this.y*this.y+this.z*this.z}length(){return Math.sqrt(this.x*this.x+this.y*this.y+this.z*this.z)}manhattanLength(){return Math.abs(this.x)+Math.abs(this.y)+Math.abs(this.z)}normalize(){return this.divideScalar(this.length()||1)}setLength(e){return this.normalize().multiplyScalar(e)}lerp(e,t){return this.x+=(e.x-this.x)*t,this.y+=(e.y-this.y)*t,this.z+=(e.z-this.z)*t,this}lerpVectors(e,t,n){return this.x=e.x+(t.x-e.x)*n,this.y=e.y+(t.y-e.y)*n,this.z=e.z+(t.z-e.z)*n,this}cross(e){return this.crossVectors(this,e)}crossVectors(e,t){let n=e.x,i=e.y,r=e.z,o=t.x,a=t.y,l=t.z;return this.x=i*l-r*a,this.y=r*o-n*l,this.z=n*a-i*o,this}projectOnVector(e){let t=e.lengthSq();if(t===0)return this.set(0,0,0);let n=e.dot(this)/t;return this.copy(e).multiplyScalar(n)}projectOnPlane(e){return Fl.copy(this).projectOnVector(e),this.sub(Fl)}reflect(e){return this.sub(Fl.copy(e).multiplyScalar(2*this.dot(e)))}angleTo(e){let t=Math.sqrt(this.lengthSq()*e.lengthSq());if(t===0)return Math.PI/2;let n=this.dot(e)/t;return Math.acos(it(n,-1,1))}distanceTo(e){return Math.sqrt(this.distanceToSquared(e))}distanceToSquared(e){let t=this.x-e.x,n=this.y-e.y,i=this.z-e.z;return t*t+n*n+i*i}manhattanDistanceTo(e){return Math.abs(this.x-e.x)+Math.abs(this.y-e.y)+Math.abs(this.z-e.z)}setFromSpherical(e){return this.setFromSphericalCoords(e.radius,e.phi,e.theta)}setFromSphericalCoords(e,t,n){let i=Math.sin(t)*e;return this.x=i*Math.sin(n),this.y=Math.cos(t)*e,this.z=i*Math.cos(n),this}setFromCylindrical(e){return this.setFromCylindricalCoords(e.radius,e.theta,e.y)}setFromCylindricalCoords(e,t,n){return this.x=e*Math.sin(t),this.y=n,this.z=e*Math.cos(t),this}setFromMatrixPosition(e){let t=e.elements;return this.x=t[12],this.y=t[13],this.z=t[14],this}setFromMatrixScale(e){let t=this.setFromMatrixColumn(e,0).length(),n=this.setFromMatrixColumn(e,1).length(),i=this.setFromMatrixColumn(e,2).length();return this.x=t,this.y=n,this.z=i,this}setFromMatrixColumn(e,t){return this.fromArray(e.elements,t*4)}setFromMatrix3Column(e,t){return this.fromArray(e.elements,t*3)}setFromEuler(e){return this.x=e._x,this.y=e._y,this.z=e._z,this}setFromColor(e){return this.x=e.r,this.y=e.g,this.z=e.b,this}equals(e){return e.x===this.x&&e.y===this.y&&e.z===this.z}fromArray(e,t=0){return this.x=e[t],this.y=e[t+1],this.z=e[t+2],this}toArray(e=[],t=0){return e[t]=this.x,e[t+1]=this.y,e[t+2]=this.z,e}fromBufferAttribute(e,t){return this.x=e.getX(t),this.y=e.getY(t),this.z=e.getZ(t),this}random(){return this.x=Math.random(),this.y=Math.random(),this.z=Math.random(),this}randomDirection(){let e=Math.random()*Math.PI*2,t=Math.random()*2-1,n=Math.sqrt(1-t*t);return this.x=n*Math.cos(e),this.y=t,this.z=n*Math.sin(e),this}*[Symbol.iterator](){yield this.x,yield this.y,yield this.z}};th.prototype.isVector3=!0;var L=th,Fl=new L,nu=new st,nh=class nh{constructor(e,t,n,i,r,o,a,l,c){this.elements=[1,0,0,0,1,0,0,0,1],e!==void 0&&this.set(e,t,n,i,r,o,a,l,c)}set(e,t,n,i,r,o,a,l,c){let u=this.elements;return u[0]=e,u[1]=i,u[2]=a,u[3]=t,u[4]=r,u[5]=l,u[6]=n,u[7]=o,u[8]=c,this}identity(){return this.set(1,0,0,0,1,0,0,0,1),this}copy(e){let t=this.elements,n=e.elements;return t[0]=n[0],t[1]=n[1],t[2]=n[2],t[3]=n[3],t[4]=n[4],t[5]=n[5],t[6]=n[6],t[7]=n[7],t[8]=n[8],this}extractBasis(e,t,n){return e.setFromMatrix3Column(this,0),t.setFromMatrix3Column(this,1),n.setFromMatrix3Column(this,2),this}setFromMatrix4(e){let t=e.elements;return this.set(t[0],t[4],t[8],t[1],t[5],t[9],t[2],t[6],t[10]),this}multiply(e){return this.multiplyMatrices(this,e)}premultiply(e){return this.multiplyMatrices(e,this)}multiplyMatrices(e,t){let n=e.elements,i=t.elements,r=this.elements,o=n[0],a=n[3],l=n[6],c=n[1],u=n[4],h=n[7],d=n[2],f=n[5],m=n[8],_=i[0],p=i[3],g=i[6],S=i[1],v=i[4],x=i[7],b=i[2],w=i[5],y=i[8];return r[0]=o*_+a*S+l*b,r[3]=o*p+a*v+l*w,r[6]=o*g+a*x+l*y,r[1]=c*_+u*S+h*b,r[4]=c*p+u*v+h*w,r[7]=c*g+u*x+h*y,r[2]=d*_+f*S+m*b,r[5]=d*p+f*v+m*w,r[8]=d*g+f*x+m*y,this}multiplyScalar(e){let t=this.elements;return t[0]*=e,t[3]*=e,t[6]*=e,t[1]*=e,t[4]*=e,t[7]*=e,t[2]*=e,t[5]*=e,t[8]*=e,this}determinant(){let e=this.elements,t=e[0],n=e[1],i=e[2],r=e[3],o=e[4],a=e[5],l=e[6],c=e[7],u=e[8];return t*o*u-t*a*c-n*r*u+n*a*l+i*r*c-i*o*l}invert(){let e=this.elements,t=e[0],n=e[1],i=e[2],r=e[3],o=e[4],a=e[5],l=e[6],c=e[7],u=e[8],h=u*o-a*c,d=a*l-u*r,f=c*r-o*l,m=t*h+n*d+i*f;if(m===0)return this.set(0,0,0,0,0,0,0,0,0);let _=1/m;return e[0]=h*_,e[1]=(i*c-u*n)*_,e[2]=(a*n-i*o)*_,e[3]=d*_,e[4]=(u*t-i*l)*_,e[5]=(i*r-a*t)*_,e[6]=f*_,e[7]=(n*l-c*t)*_,e[8]=(o*t-n*r)*_,this}transpose(){let e,t=this.elements;return e=t[1],t[1]=t[3],t[3]=e,e=t[2],t[2]=t[6],t[6]=e,e=t[5],t[5]=t[7],t[7]=e,this}getNormalMatrix(e){return this.setFromMatrix4(e).invert().transpose()}transposeIntoArray(e){let t=this.elements;return e[0]=t[0],e[1]=t[3],e[2]=t[6],e[3]=t[1],e[4]=t[4],e[5]=t[7],e[6]=t[2],e[7]=t[5],e[8]=t[8],this}setUvTransform(e,t,n,i,r,o,a){let l=Math.cos(r),c=Math.sin(r);return this.set(n*l,n*c,-n*(l*o+c*a)+o+e,-i*c,i*l,-i*(-c*o+l*a)+a+t,0,0,1),this}scale(e,t){return Vi("Matrix3: .scale() is deprecated. Use .makeScale() instead."),this.premultiply(Ol.makeScale(e,t)),this}rotate(e){return Vi("Matrix3: .rotate() is deprecated. Use .makeRotation() instead."),this.premultiply(Ol.makeRotation(-e)),this}translate(e,t){return Vi("Matrix3: .translate() is deprecated. Use .makeTranslation() instead."),this.premultiply(Ol.makeTranslation(e,t)),this}makeTranslation(e,t){return e.isVector2?this.set(1,0,e.x,0,1,e.y,0,0,1):this.set(1,0,e,0,1,t,0,0,1),this}makeRotation(e){let t=Math.cos(e),n=Math.sin(e);return this.set(t,-n,0,n,t,0,0,0,1),this}makeScale(e,t){return this.set(e,0,0,0,t,0,0,0,1),this}equals(e){let t=this.elements,n=e.elements;for(let i=0;i<9;i++)if(t[i]!==n[i])return!1;return!0}fromArray(e,t=0){for(let n=0;n<9;n++)this.elements[n]=e[n+t];return this}toArray(e=[],t=0){let n=this.elements;return e[t]=n[0],e[t+1]=n[1],e[t+2]=n[2],e[t+3]=n[3],e[t+4]=n[4],e[t+5]=n[5],e[t+6]=n[6],e[t+7]=n[7],e[t+8]=n[8],e}clone(){return new this.constructor().fromArray(this.elements)}};nh.prototype.isMatrix3=!0;var Ze=nh,Ol=new Ze,iu=new Ze().set(.4123908,.3575843,.1804808,.212639,.7151687,.0721923,.0193308,.1191948,.9505322),su=new Ze().set(3.2409699,-1.5373832,-.4986108,-.9692436,1.8759675,.0415551,.0556301,-.203977,1.0569715);function yp(){let s={enabled:!0,workingColorSpace:Gt,spaces:{},convert:function(i,r,o){return this.enabled===!1||r===o||!r||!o||(this.spaces[r].transfer===pt&&(i.r=ni(i.r),i.g=ni(i.g),i.b=ni(i.b)),this.spaces[r].primaries!==this.spaces[o].primaries&&(i.applyMatrix3(this.spaces[r].toXYZ),i.applyMatrix3(this.spaces[o].fromXYZ)),this.spaces[o].transfer===pt&&(i.r=Ss(i.r),i.g=Ss(i.g),i.b=Ss(i.b))),i},workingToColorSpace:function(i,r){return this.convert(i,this.workingColorSpace,r)},colorSpaceToWorking:function(i,r){return this.convert(i,r,this.workingColorSpace)},getPrimaries:function(i){return this.spaces[i].primaries},getTransfer:function(i){return i===qt?pr:this.spaces[i].transfer},getToneMappingMode:function(i){return this.spaces[i].outputColorSpaceConfig.toneMappingMode||"standard"},getLuminanceCoefficients:function(i,r=this.workingColorSpace){return i.fromArray(this.spaces[r].luminanceCoefficients)},define:function(i){Object.assign(this.spaces,i)},_getMatrix:function(i,r,o){return i.copy(this.spaces[o].fromXYZ).multiply(this.spaces[r].toXYZ)},_getDrawingBufferColorSpace:function(i){return this.spaces[i].outputColorSpaceConfig.drawingBufferColorSpace},_getUnpackColorSpace:function(i=this.workingColorSpace){return this.spaces[i].workingColorSpaceConfig.unpackColorSpace}},e=[.64,.33,.3,.6,.15,.06],t=[.2126,.7152,.0722],n=[.3127,.329];return s.define({[Gt]:{primaries:e,whitePoint:n,transfer:pr,toXYZ:iu,fromXYZ:su,luminanceCoefficients:t,workingColorSpaceConfig:{unpackColorSpace:wt},outputColorSpaceConfig:{drawingBufferColorSpace:wt}},[wt]:{primaries:e,whitePoint:n,transfer:pt,toXYZ:iu,fromXYZ:su,luminanceCoefficients:t,outputColorSpaceConfig:{drawingBufferColorSpace:wt}}}),s}var lt=yp();function ni(s){return s<.04045?s*.0773993808:Math.pow(s*.9478672986+.0521327014,2.4)}function Ss(s){return s<.0031308?s*12.92:1.055*Math.pow(s,.41666)-.055}var as,Qo=class{static getDataURL(e,t="image/png"){if(/^data:/i.test(e.src)||typeof HTMLCanvasElement>"u")return e.src;let n;if(e instanceof HTMLCanvasElement)n=e;else{as===void 0&&(as=Ts("canvas")),as.width=e.width,as.height=e.height;let i=as.getContext("2d");e instanceof ImageData?i.putImageData(e,0,0):i.drawImage(e,0,0,e.width,e.height),n=as}return n.toDataURL(t)}static sRGBToLinear(e){if(typeof HTMLImageElement<"u"&&e instanceof HTMLImageElement||typeof HTMLCanvasElement<"u"&&e instanceof HTMLCanvasElement||typeof ImageBitmap<"u"&&e instanceof ImageBitmap){let t=Ts("canvas");t.width=e.width,t.height=e.height;let n=t.getContext("2d");n.drawImage(e,0,0,e.width,e.height);let i=n.getImageData(0,0,e.width,e.height),r=i.data;for(let o=0;o<r.length;o++)r[o]=ni(r[o]/255)*255;return n.putImageData(i,0,0),t}else if(e.data){let t=e.data.slice(0);for(let n=0;n<t.length;n++)t instanceof Uint8Array||t instanceof Uint8ClampedArray?t[n]=Math.floor(ni(t[n]/255)*255):t[n]=ni(t[n]);return{data:t,width:e.width,height:e.height}}else return Xe("ImageUtils.sRGBToLinear(): Unsupported image type. No color space conversion applied."),e}},vp=0,Rs=class{constructor(e=null){this.isTextureSource=!0,Object.defineProperty(this,"id",{value:vp++}),this.uuid=Pi(),this.data=e,this.dataReady=!0,this.version=0}getSize(e){let t=this.data;return typeof HTMLVideoElement<"u"&&t instanceof HTMLVideoElement?e.set(t.videoWidth,t.videoHeight,0):typeof VideoFrame<"u"&&t instanceof VideoFrame?e.set(t.displayWidth,t.displayHeight,0):t!==null?e.set(t.width||0,t.height||0,t.depth||0):e.set(0,0,0),e}set needsUpdate(e){e===!0&&this.version++}toJSON(e){let t=e===void 0||typeof e=="string";if(!t&&e.images[this.uuid]!==void 0)return e.images[this.uuid];let n={uuid:this.uuid,url:""},i=this.data;if(i!==null){let r;if(Array.isArray(i)){r=[];for(let o=0,a=i.length;o<a;o++)i[o].isDataTexture?r.push(Bl(i[o].image)):r.push(Bl(i[o]))}else r=Bl(i);n.url=r}return t||(e.images[this.uuid]=n),n}};function Bl(s){return typeof HTMLImageElement<"u"&&s instanceof HTMLImageElement||typeof HTMLCanvasElement<"u"&&s instanceof HTMLCanvasElement||typeof ImageBitmap<"u"&&s instanceof ImageBitmap?Qo.getDataURL(s):s.data?{data:Array.from(s.data),width:s.width,height:s.height,type:s.data.constructor.name}:(Xe("Texture: Unable to serialize Texture."),{})}var Sp=0,zl=new L,Nt=class s extends Bn{constructor(e=s.DEFAULT_IMAGE,t=s.DEFAULT_MAPPING,n=_n,i=_n,r=Dt,o=fi,a=Mn,l=on,c=s.DEFAULT_ANISOTROPY,u=qt){super(),this.isTexture=!0,Object.defineProperty(this,"id",{value:Sp++}),this.uuid=Pi(),this.name="",this.source=new Rs(e),this.mipmaps=[],this.mapping=t,this.channel=0,this.wrapS=n,this.wrapT=i,this.magFilter=r,this.minFilter=o,this.anisotropy=c,this.format=a,this.internalFormat=null,this.type=l,this.offset=new pe(0,0),this.repeat=new pe(1,1),this.center=new pe(0,0),this.rotation=0,this.matrixAutoUpdate=!0,this.matrix=new Ze,this.generateMipmaps=!0,this.mipmapsAutoUpdate=!0,this.premultiplyAlpha=!1,this.flipY=!0,this.unpackAlignment=4,this.colorSpace=u,this.userData={},this.updateRanges=[],this.version=0,this.onUpdate=null,this.renderTarget=null,this.isRenderTargetTexture=!1,this.isArrayTexture=!!(e&&e.depth&&e.depth>1),this.pmremVersion=0,this.isPMREMTexture=!1,this.normalized=!1}get width(){return this.source.getSize(zl).x}get height(){return this.source.getSize(zl).y}get depth(){return this.source.getSize(zl).z}get image(){return this.source.data}set image(e){this.source.data=e}updateMatrix(){this.matrix.setUvTransform(this.offset.x,this.offset.y,this.repeat.x,this.repeat.y,this.rotation,this.center.x,this.center.y)}addUpdateRange(e,t){this.updateRanges.push({start:e,count:t})}clearUpdateRanges(){this.updateRanges.length=0}clone(){return new this.constructor().copy(this)}copy(e){return this.name=e.name,this.source=e.source,this.mipmaps=e.mipmaps.slice(0),this.mapping=e.mapping,this.channel=e.channel,this.wrapS=e.wrapS,this.wrapT=e.wrapT,this.magFilter=e.magFilter,this.minFilter=e.minFilter,this.anisotropy=e.anisotropy,this.format=e.format,this.internalFormat=e.internalFormat,this.type=e.type,this.normalized=e.normalized,this.offset.copy(e.offset),this.repeat.copy(e.repeat),this.center.copy(e.center),this.rotation=e.rotation,this.matrixAutoUpdate=e.matrixAutoUpdate,this.matrix.copy(e.matrix),this.generateMipmaps=e.generateMipmaps,this.mipmapsAutoUpdate=e.mipmapsAutoUpdate,this.premultiplyAlpha=e.premultiplyAlpha,this.flipY=e.flipY,this.unpackAlignment=e.unpackAlignment,this.colorSpace=e.colorSpace,this.renderTarget=e.renderTarget,this.isRenderTargetTexture=e.isRenderTargetTexture,this.isPMREMTexture=e.isPMREMTexture,this.isArrayTexture=e.isArrayTexture,this.userData=JSON.parse(JSON.stringify(e.userData)),this.needsUpdate=!0,this}setValues(e){for(let t in e){let n=e[t];if(n===void 0){Xe(`Texture.setValues(): parameter '${t}' has value of undefined.`);continue}let i=this[t];if(i===void 0){Xe(`Texture.setValues(): property '${t}' does not exist.`);continue}i&&n&&i.isVector2&&n.isVector2||i&&n&&i.isVector3&&n.isVector3||i&&n&&i.isMatrix3&&n.isMatrix3?i.copy(n):this[t]=n}}toJSON(e){let t=e===void 0||typeof e=="string";if(!t&&e.textures[this.uuid]!==void 0)return e.textures[this.uuid];let n={metadata:{version:4.7,type:"Texture",generator:"Texture.toJSON"},uuid:this.uuid,name:this.name,image:this.source.toJSON(e).uuid,mapping:this.mapping,channel:this.channel,repeat:[this.repeat.x,this.repeat.y],offset:[this.offset.x,this.offset.y],center:[this.center.x,this.center.y],rotation:this.rotation,wrap:[this.wrapS,this.wrapT],format:this.format,internalFormat:this.internalFormat,type:this.type,normalized:this.normalized,colorSpace:this.colorSpace,minFilter:this.minFilter,magFilter:this.magFilter,anisotropy:this.anisotropy,flipY:this.flipY,generateMipmaps:this.generateMipmaps,mipmapsAutoUpdate:this.mipmapsAutoUpdate,premultiplyAlpha:this.premultiplyAlpha,unpackAlignment:this.unpackAlignment};return Object.keys(this.userData).length>0&&(n.userData=this.userData),t||(e.textures[this.uuid]=n),n}dispose(){this.dispatchEvent({type:"dispose"})}transformUv(e){if(this.mapping!==Fc)return e;if(e.applyMatrix3(this.matrix),e.x<0||e.x>1)switch(this.wrapS){case Gi:e.x=e.x-Math.floor(e.x);break;case _n:e.x=e.x<0?0:1;break;case ws:Math.abs(Math.floor(e.x)%2)===1?e.x=Math.ceil(e.x)-e.x:e.x=e.x-Math.floor(e.x);break}if(e.y<0||e.y>1)switch(this.wrapT){case Gi:e.y=e.y-Math.floor(e.y);break;case _n:e.y=e.y<0?0:1;break;case ws:Math.abs(Math.floor(e.y)%2)===1?e.y=Math.ceil(e.y)-e.y:e.y=e.y-Math.floor(e.y);break}return this.flipY&&(e.y=1-e.y),e}set needsUpdate(e){e===!0&&(this.version++,this.source.needsUpdate=!0)}set needsPMREMUpdate(e){e===!0&&this.pmremVersion++}};Nt.DEFAULT_IMAGE=null;Nt.DEFAULT_MAPPING=Fc;Nt.DEFAULT_ANISOTROPY=1;var ih=class ih{constructor(e=0,t=0,n=0,i=1){this.x=e,this.y=t,this.z=n,this.w=i}get width(){return this.z}set width(e){this.z=e}get height(){return this.w}set height(e){this.w=e}set(e,t,n,i){return this.x=e,this.y=t,this.z=n,this.w=i,this}setScalar(e){return this.x=e,this.y=e,this.z=e,this.w=e,this}setX(e){return this.x=e,this}setY(e){return this.y=e,this}setZ(e){return this.z=e,this}setW(e){return this.w=e,this}setComponent(e,t){switch(e){case 0:this.x=t;break;case 1:this.y=t;break;case 2:this.z=t;break;case 3:this.w=t;break;default:throw new Error("THREE.Vector4: index is out of range: "+e)}return this}getComponent(e){switch(e){case 0:return this.x;case 1:return this.y;case 2:return this.z;case 3:return this.w;default:throw new Error("THREE.Vector4: index is out of range: "+e)}}clone(){return new this.constructor(this.x,this.y,this.z,this.w)}copy(e){return this.x=e.x,this.y=e.y,this.z=e.z,this.w=e.w!==void 0?e.w:1,this}add(e){return this.x+=e.x,this.y+=e.y,this.z+=e.z,this.w+=e.w,this}addScalar(e){return this.x+=e,this.y+=e,this.z+=e,this.w+=e,this}addVectors(e,t){return this.x=e.x+t.x,this.y=e.y+t.y,this.z=e.z+t.z,this.w=e.w+t.w,this}addScaledVector(e,t){return this.x+=e.x*t,this.y+=e.y*t,this.z+=e.z*t,this.w+=e.w*t,this}sub(e){return this.x-=e.x,this.y-=e.y,this.z-=e.z,this.w-=e.w,this}subScalar(e){return this.x-=e,this.y-=e,this.z-=e,this.w-=e,this}subVectors(e,t){return this.x=e.x-t.x,this.y=e.y-t.y,this.z=e.z-t.z,this.w=e.w-t.w,this}multiply(e){return this.x*=e.x,this.y*=e.y,this.z*=e.z,this.w*=e.w,this}multiplyScalar(e){return this.x*=e,this.y*=e,this.z*=e,this.w*=e,this}applyMatrix4(e){let t=this.x,n=this.y,i=this.z,r=this.w,o=e.elements;return this.x=o[0]*t+o[4]*n+o[8]*i+o[12]*r,this.y=o[1]*t+o[5]*n+o[9]*i+o[13]*r,this.z=o[2]*t+o[6]*n+o[10]*i+o[14]*r,this.w=o[3]*t+o[7]*n+o[11]*i+o[15]*r,this}divide(e){return this.x/=e.x,this.y/=e.y,this.z/=e.z,this.w/=e.w,this}divideScalar(e){return this.multiplyScalar(1/e)}setAxisAngleFromQuaternion(e){this.w=2*Math.acos(e.w);let t=Math.sqrt(1-e.w*e.w);return t<1e-4?(this.x=1,this.y=0,this.z=0):(this.x=e.x/t,this.y=e.y/t,this.z=e.z/t),this}setAxisAngleFromRotationMatrix(e){let t,n,i,r,l=e.elements,c=l[0],u=l[4],h=l[8],d=l[1],f=l[5],m=l[9],_=l[2],p=l[6],g=l[10];if(Math.abs(u-d)<.01&&Math.abs(h-_)<.01&&Math.abs(m-p)<.01){if(Math.abs(u+d)<.1&&Math.abs(h+_)<.1&&Math.abs(m+p)<.1&&Math.abs(c+f+g-3)<.1)return this.set(1,0,0,0),this;t=Math.PI;let v=(c+1)/2,x=(f+1)/2,b=(g+1)/2,w=(u+d)/4,y=(h+_)/4,M=(m+p)/4;return v>x&&v>b?v<.01?(n=0,i=.707106781,r=.707106781):(n=Math.sqrt(v),i=w/n,r=y/n):x>b?x<.01?(n=.707106781,i=0,r=.707106781):(i=Math.sqrt(x),n=w/i,r=M/i):b<.01?(n=.707106781,i=.707106781,r=0):(r=Math.sqrt(b),n=y/r,i=M/r),this.set(n,i,r,t),this}let S=Math.sqrt((p-m)*(p-m)+(h-_)*(h-_)+(d-u)*(d-u));return Math.abs(S)<.001&&(S=1),this.x=(p-m)/S,this.y=(h-_)/S,this.z=(d-u)/S,this.w=Math.acos((c+f+g-1)/2),this}setFromMatrixPosition(e){let t=e.elements;return this.x=t[12],this.y=t[13],this.z=t[14],this.w=t[15],this}min(e){return this.x=Math.min(this.x,e.x),this.y=Math.min(this.y,e.y),this.z=Math.min(this.z,e.z),this.w=Math.min(this.w,e.w),this}max(e){return this.x=Math.max(this.x,e.x),this.y=Math.max(this.y,e.y),this.z=Math.max(this.z,e.z),this.w=Math.max(this.w,e.w),this}clamp(e,t){return this.x=it(this.x,e.x,t.x),this.y=it(this.y,e.y,t.y),this.z=it(this.z,e.z,t.z),this.w=it(this.w,e.w,t.w),this}clampScalar(e,t){return this.x=it(this.x,e,t),this.y=it(this.y,e,t),this.z=it(this.z,e,t),this.w=it(this.w,e,t),this}clampLength(e,t){let n=this.length();return this.divideScalar(n||1).multiplyScalar(it(n,e,t))}floor(){return this.x=Math.floor(this.x),this.y=Math.floor(this.y),this.z=Math.floor(this.z),this.w=Math.floor(this.w),this}ceil(){return this.x=Math.ceil(this.x),this.y=Math.ceil(this.y),this.z=Math.ceil(this.z),this.w=Math.ceil(this.w),this}round(){return this.x=Math.round(this.x),this.y=Math.round(this.y),this.z=Math.round(this.z),this.w=Math.round(this.w),this}roundToZero(){return this.x=Math.trunc(this.x),this.y=Math.trunc(this.y),this.z=Math.trunc(this.z),this.w=Math.trunc(this.w),this}negate(){return this.x=-this.x,this.y=-this.y,this.z=-this.z,this.w=-this.w,this}dot(e){return this.x*e.x+this.y*e.y+this.z*e.z+this.w*e.w}lengthSq(){return this.x*this.x+this.y*this.y+this.z*this.z+this.w*this.w}length(){return Math.sqrt(this.x*this.x+this.y*this.y+this.z*this.z+this.w*this.w)}manhattanLength(){return Math.abs(this.x)+Math.abs(this.y)+Math.abs(this.z)+Math.abs(this.w)}normalize(){return this.divideScalar(this.length()||1)}setLength(e){return this.normalize().multiplyScalar(e)}lerp(e,t){return this.x+=(e.x-this.x)*t,this.y+=(e.y-this.y)*t,this.z+=(e.z-this.z)*t,this.w+=(e.w-this.w)*t,this}lerpVectors(e,t,n){return this.x=e.x+(t.x-e.x)*n,this.y=e.y+(t.y-e.y)*n,this.z=e.z+(t.z-e.z)*n,this.w=e.w+(t.w-e.w)*n,this}equals(e){return e.x===this.x&&e.y===this.y&&e.z===this.z&&e.w===this.w}fromArray(e,t=0){return this.x=e[t],this.y=e[t+1],this.z=e[t+2],this.w=e[t+3],this}toArray(e=[],t=0){return e[t]=this.x,e[t+1]=this.y,e[t+2]=this.z,e[t+3]=this.w,e}fromBufferAttribute(e,t){return this.x=e.getX(t),this.y=e.getY(t),this.z=e.getZ(t),this.w=e.getW(t),this}random(){return this.x=Math.random(),this.y=Math.random(),this.z=Math.random(),this.w=Math.random(),this}*[Symbol.iterator](){yield this.x,yield this.y,yield this.z,yield this.w}};ih.prototype.isVector4=!0;var _t=ih,ea=class extends Bn{constructor(e=1,t=1,n={}){super(),n=Object.assign({generateMipmaps:!1,internalFormat:null,minFilter:Dt,depthBuffer:!0,stencilBuffer:!1,resolveColorBuffer:!0,resolveDepthBuffer:!0,resolveStencilBuffer:!0,storeMultisampledColorBuffer:!0,storeMultisampledDepthBuffer:!0,storeMultisampledStencilBuffer:!0,depthTexture:null,samples:0,count:1,depth:1,multiview:!1,useArrayDepthTexture:!1},n),this.isRenderTarget=!0,this.width=e,this.height=t,this.depth=n.depth,this.scissor=new _t(0,0,e,t),this.scissorTest=!1,this.viewport=new _t(0,0,e,t),this.textures=[];let i=n.count;for(let r=0;r<i;r++)this.textures[r]=new Nt({width:e,height:t,depth:n.depth}),this.textures[r].isRenderTargetTexture=!0,this.textures[r].renderTarget=this;this._setTextureOptions(n),this.depthBuffer=n.depthBuffer,this.stencilBuffer=n.stencilBuffer,this.resolveColorBuffer=n.resolveColorBuffer,this.resolveDepthBuffer=n.resolveDepthBuffer,this.resolveStencilBuffer=n.resolveStencilBuffer,this.storeMultisampledColorBuffer=n.storeMultisampledColorBuffer,this.storeMultisampledDepthBuffer=n.storeMultisampledDepthBuffer,this.storeMultisampledStencilBuffer=n.storeMultisampledStencilBuffer,this._depthTexture=null,this.depthTexture=n.depthTexture,this.samples=n.samples,this.multiview=n.multiview,this.useArrayDepthTexture=n.useArrayDepthTexture}_setTextureOptions(e={}){let t={minFilter:Dt,generateMipmaps:!1,flipY:!1,internalFormat:null};e.mapping!==void 0&&(t.mapping=e.mapping),e.wrapS!==void 0&&(t.wrapS=e.wrapS),e.wrapT!==void 0&&(t.wrapT=e.wrapT),e.wrapR!==void 0&&(t.wrapR=e.wrapR),e.magFilter!==void 0&&(t.magFilter=e.magFilter),e.minFilter!==void 0&&(t.minFilter=e.minFilter),e.format!==void 0&&(t.format=e.format),e.type!==void 0&&(t.type=e.type),e.anisotropy!==void 0&&(t.anisotropy=e.anisotropy),e.colorSpace!==void 0&&(t.colorSpace=e.colorSpace),e.flipY!==void 0&&(t.flipY=e.flipY),e.generateMipmaps!==void 0&&(t.generateMipmaps=e.generateMipmaps),e.mipmapsAutoUpdate!==void 0&&(t.mipmapsAutoUpdate=e.mipmapsAutoUpdate),e.internalFormat!==void 0&&(t.internalFormat=e.internalFormat);for(let n=0;n<this.textures.length;n++)this.textures[n].setValues(t)}get texture(){return this.textures[0]}set texture(e){this.textures[0]=e}set depthTexture(e){this._depthTexture!==null&&this._depthTexture.renderTarget===this&&(this._depthTexture.renderTarget=null),e!==null&&e.renderTarget===null&&(e.renderTarget=this),this._depthTexture=e}get depthTexture(){return this._depthTexture}setSize(e,t,n=1){if(this.width!==e||this.height!==t||this.depth!==n){this.width=e,this.height=t,this.depth=n;for(let i=0,r=this.textures.length;i<r;i++)this.textures[i].image.width=e,this.textures[i].image.height=t,this.textures[i].image.depth=n,this.textures[i].isData3DTexture!==!0&&(this.textures[i].isArrayTexture=this.textures[i].image.depth>1);this.dispose()}this.viewport.set(0,0,e,t),this.scissor.set(0,0,e,t)}clone(){return new this.constructor().copy(this)}copy(e){this.width=e.width,this.height=e.height,this.depth=e.depth,this.scissor.copy(e.scissor),this.scissorTest=e.scissorTest,this.viewport.copy(e.viewport),this.textures.length=0;for(let t=0,n=e.textures.length;t<n;t++){this.textures[t]=e.textures[t].clone(),this.textures[t].isRenderTargetTexture=!0,this.textures[t].renderTarget=this;let i=Object.assign({},e.textures[t].image);this.textures[t].source=new Rs(i)}if(this.depthBuffer=e.depthBuffer,this.stencilBuffer=e.stencilBuffer,this.resolveColorBuffer=e.resolveColorBuffer,this.resolveDepthBuffer=e.resolveDepthBuffer,this.resolveStencilBuffer=e.resolveStencilBuffer,this.storeMultisampledColorBuffer=e.storeMultisampledColorBuffer,this.storeMultisampledDepthBuffer=e.storeMultisampledDepthBuffer,this.storeMultisampledStencilBuffer=e.storeMultisampledStencilBuffer,e.depthTexture!==null)if(e.depthTexture.renderTarget===e){let t=e.depthTexture.clone();t.renderTarget=null,this.depthTexture=t}else this.depthTexture=e.depthTexture;else this.depthTexture=null;return this.samples=e.samples,this.multiview=e.multiview,this.useArrayDepthTexture=e.useArrayDepthTexture,this}dispose(){this.dispatchEvent({type:"dispose"})}},hn=class extends ea{constructor(e=1,t=1,n={}){super(e,t,n),this.isWebGLRenderTarget=!0}},gr=class extends Nt{constructor(e=null,t=1,n=1,i=1){super(null),this.isDataArrayTexture=!0,this.image={data:e,width:t,height:n,depth:i},this.magFilter=Vt,this.minFilter=Vt,this.wrapR=_n,this.generateMipmaps=!1,this.flipY=!1,this.unpackAlignment=1,this.layerUpdates=new Set}copy(e){return super.copy(e),this.wrapR=e.wrapR,this}addLayerUpdate(e){this.layerUpdates.add(e)}clearLayerUpdates(){this.layerUpdates.clear()}};var ta=class extends Nt{constructor(e=null,t=1,n=1,i=1){super(null),this.isData3DTexture=!0,this.image={data:e,width:t,height:n,depth:i},this.magFilter=Vt,this.minFilter=Vt,this.wrapR=_n,this.generateMipmaps=!1,this.flipY=!1,this.unpackAlignment=1}copy(e){return super.copy(e),this.wrapR=e.wrapR,this}};var ya=class ya{constructor(e,t,n,i,r,o,a,l,c,u,h,d,f,m,_,p){this.elements=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1],e!==void 0&&this.set(e,t,n,i,r,o,a,l,c,u,h,d,f,m,_,p)}set(e,t,n,i,r,o,a,l,c,u,h,d,f,m,_,p){let g=this.elements;return g[0]=e,g[4]=t,g[8]=n,g[12]=i,g[1]=r,g[5]=o,g[9]=a,g[13]=l,g[2]=c,g[6]=u,g[10]=h,g[14]=d,g[3]=f,g[7]=m,g[11]=_,g[15]=p,this}identity(){return this.set(1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1),this}clone(){return new ya().fromArray(this.elements)}copy(e){let t=this.elements,n=e.elements;return t[0]=n[0],t[1]=n[1],t[2]=n[2],t[3]=n[3],t[4]=n[4],t[5]=n[5],t[6]=n[6],t[7]=n[7],t[8]=n[8],t[9]=n[9],t[10]=n[10],t[11]=n[11],t[12]=n[12],t[13]=n[13],t[14]=n[14],t[15]=n[15],this}copyPosition(e){let t=this.elements,n=e.elements;return t[12]=n[12],t[13]=n[13],t[14]=n[14],this}setFromMatrix3(e){let t=e.elements;return this.set(t[0],t[3],t[6],0,t[1],t[4],t[7],0,t[2],t[5],t[8],0,0,0,0,1),this}extractBasis(e,t,n){return this.determinantAffine()===0?(e.set(1,0,0),t.set(0,1,0),n.set(0,0,1),this):(e.setFromMatrixColumn(this,0),t.setFromMatrixColumn(this,1),n.setFromMatrixColumn(this,2),this)}makeBasis(e,t,n){return this.set(e.x,t.x,n.x,0,e.y,t.y,n.y,0,e.z,t.z,n.z,0,0,0,0,1),this}extractRotation(e){if(e.determinantAffine()===0)return this.identity();let t=this.elements,n=e.elements,i=1/ls.setFromMatrixColumn(e,0).length(),r=1/ls.setFromMatrixColumn(e,1).length(),o=1/ls.setFromMatrixColumn(e,2).length();return t[0]=n[0]*i,t[1]=n[1]*i,t[2]=n[2]*i,t[3]=0,t[4]=n[4]*r,t[5]=n[5]*r,t[6]=n[6]*r,t[7]=0,t[8]=n[8]*o,t[9]=n[9]*o,t[10]=n[10]*o,t[11]=0,t[12]=0,t[13]=0,t[14]=0,t[15]=1,this}makeRotationFromEuler(e){let t=this.elements,n=e.x,i=e.y,r=e.z,o=Math.cos(n),a=Math.sin(n),l=Math.cos(i),c=Math.sin(i),u=Math.cos(r),h=Math.sin(r);if(e.order==="XYZ"){let d=o*u,f=o*h,m=a*u,_=a*h;t[0]=l*u,t[4]=-l*h,t[8]=c,t[1]=f+m*c,t[5]=d-_*c,t[9]=-a*l,t[2]=_-d*c,t[6]=m+f*c,t[10]=o*l}else if(e.order==="YXZ"){let d=l*u,f=l*h,m=c*u,_=c*h;t[0]=d+_*a,t[4]=m*a-f,t[8]=o*c,t[1]=o*h,t[5]=o*u,t[9]=-a,t[2]=f*a-m,t[6]=_+d*a,t[10]=o*l}else if(e.order==="ZXY"){let d=l*u,f=l*h,m=c*u,_=c*h;t[0]=d-_*a,t[4]=-o*h,t[8]=m+f*a,t[1]=f+m*a,t[5]=o*u,t[9]=_-d*a,t[2]=-o*c,t[6]=a,t[10]=o*l}else if(e.order==="ZYX"){let d=o*u,f=o*h,m=a*u,_=a*h;t[0]=l*u,t[4]=m*c-f,t[8]=d*c+_,t[1]=l*h,t[5]=_*c+d,t[9]=f*c-m,t[2]=-c,t[6]=a*l,t[10]=o*l}else if(e.order==="YZX"){let d=o*l,f=o*c,m=a*l,_=a*c;t[0]=l*u,t[4]=_-d*h,t[8]=m*h+f,t[1]=h,t[5]=o*u,t[9]=-a*u,t[2]=-c*u,t[6]=f*h+m,t[10]=d-_*h}else if(e.order==="XZY"){let d=o*l,f=o*c,m=a*l,_=a*c;t[0]=l*u,t[4]=-h,t[8]=c*u,t[1]=d*h+_,t[5]=o*u,t[9]=f*h-m,t[2]=m*h-f,t[6]=a*u,t[10]=_*h+d}return t[3]=0,t[7]=0,t[11]=0,t[12]=0,t[13]=0,t[14]=0,t[15]=1,this}makeRotationFromQuaternion(e){return this.compose(Mp,e,bp)}lookAt(e,t,n){let i=this.elements;return an.subVectors(e,t),an.lengthSq()===0&&(an.z=1),an.normalize(),xi.crossVectors(n,an),xi.lengthSq()===0&&(Math.abs(n.z)===1?an.x+=1e-4:an.z+=1e-4,an.normalize(),xi.crossVectors(n,an)),xi.normalize(),vo.crossVectors(an,xi),i[0]=xi.x,i[4]=vo.x,i[8]=an.x,i[1]=xi.y,i[5]=vo.y,i[9]=an.y,i[2]=xi.z,i[6]=vo.z,i[10]=an.z,this}multiply(e){return this.multiplyMatrices(this,e)}premultiply(e){return this.multiplyMatrices(e,this)}multiplyMatrices(e,t){let n=e.elements,i=t.elements,r=this.elements,o=n[0],a=n[4],l=n[8],c=n[12],u=n[1],h=n[5],d=n[9],f=n[13],m=n[2],_=n[6],p=n[10],g=n[14],S=n[3],v=n[7],x=n[11],b=n[15],w=i[0],y=i[4],M=i[8],C=i[12],E=i[1],R=i[5],D=i[9],k=i[13],P=i[2],O=i[6],G=i[10],U=i[14],ee=i[3],W=i[7],K=i[11],$=i[15];return r[0]=o*w+a*E+l*P+c*ee,r[4]=o*y+a*R+l*O+c*W,r[8]=o*M+a*D+l*G+c*K,r[12]=o*C+a*k+l*U+c*$,r[1]=u*w+h*E+d*P+f*ee,r[5]=u*y+h*R+d*O+f*W,r[9]=u*M+h*D+d*G+f*K,r[13]=u*C+h*k+d*U+f*$,r[2]=m*w+_*E+p*P+g*ee,r[6]=m*y+_*R+p*O+g*W,r[10]=m*M+_*D+p*G+g*K,r[14]=m*C+_*k+p*U+g*$,r[3]=S*w+v*E+x*P+b*ee,r[7]=S*y+v*R+x*O+b*W,r[11]=S*M+v*D+x*G+b*K,r[15]=S*C+v*k+x*U+b*$,this}multiplyScalar(e){let t=this.elements;return t[0]*=e,t[4]*=e,t[8]*=e,t[12]*=e,t[1]*=e,t[5]*=e,t[9]*=e,t[13]*=e,t[2]*=e,t[6]*=e,t[10]*=e,t[14]*=e,t[3]*=e,t[7]*=e,t[11]*=e,t[15]*=e,this}determinant(){let e=this.elements,t=e[0],n=e[4],i=e[8],r=e[12],o=e[1],a=e[5],l=e[9],c=e[13],u=e[2],h=e[6],d=e[10],f=e[14],m=e[3],_=e[7],p=e[11],g=e[15],S=l*f-c*d,v=a*f-c*h,x=a*d-l*h,b=o*f-c*u,w=o*d-l*u,y=o*h-a*u;return t*(_*S-p*v+g*x)-n*(m*S-p*b+g*w)+i*(m*v-_*b+g*y)-r*(m*x-_*w+p*y)}determinantAffine(){let e=this.elements,t=e[0],n=e[4],i=e[8],r=e[1],o=e[5],a=e[9],l=e[2],c=e[6],u=e[10];return t*(o*u-a*c)-n*(r*u-a*l)+i*(r*c-o*l)}transpose(){let e=this.elements,t;return t=e[1],e[1]=e[4],e[4]=t,t=e[2],e[2]=e[8],e[8]=t,t=e[6],e[6]=e[9],e[9]=t,t=e[3],e[3]=e[12],e[12]=t,t=e[7],e[7]=e[13],e[13]=t,t=e[11],e[11]=e[14],e[14]=t,this}setPosition(e,t,n){let i=this.elements;return e.isVector3?(i[12]=e.x,i[13]=e.y,i[14]=e.z):(i[12]=e,i[13]=t,i[14]=n),this}invert(){let e=this.elements,t=e[0],n=e[1],i=e[2],r=e[3],o=e[4],a=e[5],l=e[6],c=e[7],u=e[8],h=e[9],d=e[10],f=e[11],m=e[12],_=e[13],p=e[14],g=e[15],S=t*a-n*o,v=t*l-i*o,x=t*c-r*o,b=n*l-i*a,w=n*c-r*a,y=i*c-r*l,M=u*_-h*m,C=u*p-d*m,E=u*g-f*m,R=h*p-d*_,D=h*g-f*_,k=d*g-f*p,P=S*k-v*D+x*R+b*E-w*C+y*M;if(P===0)return this.set(0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0);let O=1/P;return e[0]=(a*k-l*D+c*R)*O,e[1]=(i*D-n*k-r*R)*O,e[2]=(_*y-p*w+g*b)*O,e[3]=(d*w-h*y-f*b)*O,e[4]=(l*E-o*k-c*C)*O,e[5]=(t*k-i*E+r*C)*O,e[6]=(p*x-m*y-g*v)*O,e[7]=(u*y-d*x+f*v)*O,e[8]=(o*D-a*E+c*M)*O,e[9]=(n*E-t*D-r*M)*O,e[10]=(m*w-_*x+g*S)*O,e[11]=(h*x-u*w-f*S)*O,e[12]=(a*C-o*R-l*M)*O,e[13]=(t*R-n*C+i*M)*O,e[14]=(_*v-m*b-p*S)*O,e[15]=(u*b-h*v+d*S)*O,this}scale(e){let t=this.elements,n=e.x,i=e.y,r=e.z;return t[0]*=n,t[4]*=i,t[8]*=r,t[1]*=n,t[5]*=i,t[9]*=r,t[2]*=n,t[6]*=i,t[10]*=r,t[3]*=n,t[7]*=i,t[11]*=r,this}getMaxScaleOnAxis(){let e=this.elements,t=e[0]*e[0]+e[1]*e[1]+e[2]*e[2],n=e[4]*e[4]+e[5]*e[5]+e[6]*e[6],i=e[8]*e[8]+e[9]*e[9]+e[10]*e[10];return Math.sqrt(Math.max(t,n,i))}makeTranslation(e,t,n){return e.isVector3?this.set(1,0,0,e.x,0,1,0,e.y,0,0,1,e.z,0,0,0,1):this.set(1,0,0,e,0,1,0,t,0,0,1,n,0,0,0,1),this}makeRotationX(e){let t=Math.cos(e),n=Math.sin(e);return this.set(1,0,0,0,0,t,-n,0,0,n,t,0,0,0,0,1),this}makeRotationY(e){let t=Math.cos(e),n=Math.sin(e);return this.set(t,0,n,0,0,1,0,0,-n,0,t,0,0,0,0,1),this}makeRotationZ(e){let t=Math.cos(e),n=Math.sin(e);return this.set(t,-n,0,0,n,t,0,0,0,0,1,0,0,0,0,1),this}makeRotationAxis(e,t){let n=Math.cos(t),i=Math.sin(t),r=1-n,o=e.x,a=e.y,l=e.z,c=r*o,u=r*a;return this.set(c*o+n,c*a-i*l,c*l+i*a,0,c*a+i*l,u*a+n,u*l-i*o,0,c*l-i*a,u*l+i*o,r*l*l+n,0,0,0,0,1),this}makeScale(e,t,n){return this.set(e,0,0,0,0,t,0,0,0,0,n,0,0,0,0,1),this}makeShear(e,t,n,i,r,o){return this.set(1,n,r,0,e,1,o,0,t,i,1,0,0,0,0,1),this}compose(e,t,n){let i=this.elements,r=t._x,o=t._y,a=t._z,l=t._w,c=r+r,u=o+o,h=a+a,d=r*c,f=r*u,m=r*h,_=o*u,p=o*h,g=a*h,S=l*c,v=l*u,x=l*h,b=n.x,w=n.y,y=n.z;return i[0]=(1-(_+g))*b,i[1]=(f+x)*b,i[2]=(m-v)*b,i[3]=0,i[4]=(f-x)*w,i[5]=(1-(d+g))*w,i[6]=(p+S)*w,i[7]=0,i[8]=(m+v)*y,i[9]=(p-S)*y,i[10]=(1-(d+_))*y,i[11]=0,i[12]=e.x,i[13]=e.y,i[14]=e.z,i[15]=1,this}decompose(e,t,n){let i=this.elements;e.x=i[12],e.y=i[13],e.z=i[14];let r=this.determinantAffine();if(r===0)return n.set(1,1,1),t.identity(),this;let o=ls.set(i[0],i[1],i[2]).length(),a=ls.set(i[4],i[5],i[6]).length(),l=ls.set(i[8],i[9],i[10]).length();r<0&&(o=-o),An.copy(this);let c=1/o,u=1/a,h=1/l;return An.elements[0]*=c,An.elements[1]*=c,An.elements[2]*=c,An.elements[4]*=u,An.elements[5]*=u,An.elements[6]*=u,An.elements[8]*=h,An.elements[9]*=h,An.elements[10]*=h,t.setFromRotationMatrix(An),n.x=o,n.y=a,n.z=l,this}makePerspective(e,t,n,i,r,o,a=xn,l=!1){let c=this.elements,u=2*r/(t-e),h=2*r/(n-i),d=(t+e)/(t-e),f=(n+i)/(n-i),m,_;if(l)m=r/(o-r),_=o*r/(o-r);else if(a===xn)m=-(o+r)/(o-r),_=-2*o*r/(o-r);else if(a===As)m=-o/(o-r),_=-o*r/(o-r);else throw new Error("THREE.Matrix4.makePerspective(): Invalid coordinate system: "+a);return c[0]=u,c[4]=0,c[8]=d,c[12]=0,c[1]=0,c[5]=h,c[9]=f,c[13]=0,c[2]=0,c[6]=0,c[10]=m,c[14]=_,c[3]=0,c[7]=0,c[11]=-1,c[15]=0,this}makeOrthographic(e,t,n,i,r,o,a=xn,l=!1){let c=this.elements,u=2/(t-e),h=2/(n-i),d=-(t+e)/(t-e),f=-(n+i)/(n-i),m,_;if(l)m=1/(o-r),_=o/(o-r);else if(a===xn)m=-2/(o-r),_=-(o+r)/(o-r);else if(a===As)m=-1/(o-r),_=-r/(o-r);else throw new Error("THREE.Matrix4.makeOrthographic(): Invalid coordinate system: "+a);return c[0]=u,c[4]=0,c[8]=0,c[12]=d,c[1]=0,c[5]=h,c[9]=0,c[13]=f,c[2]=0,c[6]=0,c[10]=m,c[14]=_,c[3]=0,c[7]=0,c[11]=0,c[15]=1,this}equals(e){let t=this.elements,n=e.elements;for(let i=0;i<16;i++)if(t[i]!==n[i])return!1;return!0}fromArray(e,t=0){for(let n=0;n<16;n++)this.elements[n]=e[n+t];return this}toArray(e=[],t=0){let n=this.elements;return e[t]=n[0],e[t+1]=n[1],e[t+2]=n[2],e[t+3]=n[3],e[t+4]=n[4],e[t+5]=n[5],e[t+6]=n[6],e[t+7]=n[7],e[t+8]=n[8],e[t+9]=n[9],e[t+10]=n[10],e[t+11]=n[11],e[t+12]=n[12],e[t+13]=n[13],e[t+14]=n[14],e[t+15]=n[15],e}};ya.prototype.isMatrix4=!0;var ze=ya,ls=new L,An=new ze,Mp=new L(0,0,0),bp=new L(1,1,1),xi=new L,vo=new L,an=new L,ru=new ze,ou=new st,Ut=class s{constructor(e=0,t=0,n=0,i=s.DEFAULT_ORDER){this.isEuler=!0,this._x=e,this._y=t,this._z=n,this._order=i}get x(){return this._x}set x(e){this._x=e,this._onChangeCallback()}get y(){return this._y}set y(e){this._y=e,this._onChangeCallback()}get z(){return this._z}set z(e){this._z=e,this._onChangeCallback()}get order(){return this._order}set order(e){this._order=e,this._onChangeCallback()}set(e,t,n,i=this._order){return this._x=e,this._y=t,this._z=n,this._order=i,this._onChangeCallback(),this}clone(){return new this.constructor(this._x,this._y,this._z,this._order)}copy(e){return this._x=e._x,this._y=e._y,this._z=e._z,this._order=e._order,this._onChangeCallback(),this}setFromRotationMatrix(e,t=this._order,n=!0){let i=e.elements,r=i[0],o=i[4],a=i[8],l=i[1],c=i[5],u=i[9],h=i[2],d=i[6],f=i[10];switch(t){case"XYZ":this._y=Math.asin(it(a,-1,1)),Math.abs(a)<.9999999?(this._x=Math.atan2(-u,f),this._z=Math.atan2(-o,r)):(this._x=Math.atan2(d,c),this._z=0);break;case"YXZ":this._x=Math.asin(-it(u,-1,1)),Math.abs(u)<.9999999?(this._y=Math.atan2(a,f),this._z=Math.atan2(l,c)):(this._y=Math.atan2(-h,r),this._z=0);break;case"ZXY":this._x=Math.asin(it(d,-1,1)),Math.abs(d)<.9999999?(this._y=Math.atan2(-h,f),this._z=Math.atan2(-o,c)):(this._y=0,this._z=Math.atan2(l,r));break;case"ZYX":this._y=Math.asin(-it(h,-1,1)),Math.abs(h)<.9999999?(this._x=Math.atan2(d,f),this._z=Math.atan2(l,r)):(this._x=0,this._z=Math.atan2(-o,c));break;case"YZX":this._z=Math.asin(it(l,-1,1)),Math.abs(l)<.9999999?(this._x=Math.atan2(-u,c),this._y=Math.atan2(-h,r)):(this._x=0,this._y=Math.atan2(a,f));break;case"XZY":this._z=Math.asin(-it(o,-1,1)),Math.abs(o)<.9999999?(this._x=Math.atan2(d,c),this._y=Math.atan2(a,r)):(this._x=Math.atan2(-u,f),this._y=0);break;default:Xe("Euler: .setFromRotationMatrix() encountered an unknown order: "+t)}return this._order=t,n===!0&&this._onChangeCallback(),this}setFromQuaternion(e,t,n){return ru.makeRotationFromQuaternion(e),this.setFromRotationMatrix(ru,t,n)}setFromVector3(e,t=this._order){return this.set(e.x,e.y,e.z,t)}reorder(e){return ou.setFromEuler(this),this.setFromQuaternion(ou,e)}equals(e){return e._x===this._x&&e._y===this._y&&e._z===this._z&&e._order===this._order}fromArray(e){return this._x=e[0],this._y=e[1],this._z=e[2],e[3]!==void 0&&(this._order=e[3]),this._onChangeCallback(),this}toArray(e=[],t=0){return e[t]=this._x,e[t+1]=this._y,e[t+2]=this._z,e[t+3]=this._order,e}_onChange(e){return this._onChangeCallback=e,this}_onChangeCallback(){}*[Symbol.iterator](){yield this._x,yield this._y,yield this._z,yield this._order}};Ut.DEFAULT_ORDER="XYZ";var Cs=class{constructor(){this.mask=1}set(e){this.mask=(1<<e|0)>>>0}enable(e){this.mask|=1<<e|0}enableAll(){this.mask=-1}toggle(e){this.mask^=1<<e|0}disable(e){this.mask&=~(1<<e|0)}disableAll(){this.mask=0}test(e){return(this.mask&e.mask)!==0}isEnabled(e){return(this.mask&(1<<e|0))!==0}},wp=0,au=new L,cs=new st,$n=new ze,So=new L,nr=new L,Ap=new L,Tp=new st,lu=new L(1,0,0),cu=new L(0,1,0),hu=new L(0,0,1),uu={type:"added"},Ep={type:"removed"},hs={type:"childadded",child:null},kl={type:"childremoved",child:null},At=class s extends Bn{constructor(){super(),this.isObject3D=!0,Object.defineProperty(this,"id",{value:wp++}),this.uuid=Pi(),this.name="",this.type="Object3D",this.parent=null,this.children=[],this.up=s.DEFAULT_UP.clone();let e=new L,t=new Ut,n=new st,i=new L(1,1,1);function r(){n.setFromEuler(t,!1)}function o(){t.setFromQuaternion(n,void 0,!1)}t._onChange(r),n._onChange(o),Object.defineProperties(this,{position:{configurable:!0,enumerable:!0,value:e},rotation:{configurable:!0,enumerable:!0,value:t},quaternion:{configurable:!0,enumerable:!0,value:n},scale:{configurable:!0,enumerable:!0,value:i},modelViewMatrix:{value:new ze},normalMatrix:{value:new Ze}}),this.matrix=new ze,this.matrixWorld=new ze,this.matrixAutoUpdate=s.DEFAULT_MATRIX_AUTO_UPDATE,this.matrixWorldAutoUpdate=s.DEFAULT_MATRIX_WORLD_AUTO_UPDATE,this.matrixWorldNeedsUpdate=!1,this.layers=new Cs,this.visible=!0,this.castShadow=!1,this.receiveShadow=!1,this.frustumCulled=!0,this.renderOrder=0,this.animations=[],this.customDepthMaterial=void 0,this.customDistanceMaterial=void 0,this.static=!1,this.userData={},this.pivot=null}onBeforeShadow(){}onAfterShadow(){}onBeforeRender(){}onAfterRender(){}applyMatrix4(e){this.matrixAutoUpdate&&this.updateMatrix(),this.matrix.premultiply(e),this.matrix.decompose(this.position,this.quaternion,this.scale),this.pivot!==null&&this.position.copy(this.pivot).applyMatrix4(this.matrix).sub(this.pivot)}applyQuaternion(e){return this.quaternion.premultiply(e),this}setRotationFromAxisAngle(e,t){this.quaternion.setFromAxisAngle(e,t)}setRotationFromEuler(e){this.quaternion.setFromEuler(e,!0)}setRotationFromMatrix(e){this.quaternion.setFromRotationMatrix(e)}setRotationFromQuaternion(e){this.quaternion.copy(e)}rotateOnAxis(e,t){return cs.setFromAxisAngle(e,t),this.quaternion.multiply(cs),this}rotateOnWorldAxis(e,t){return cs.setFromAxisAngle(e,t),this.quaternion.premultiply(cs),this}rotateX(e){return this.rotateOnAxis(lu,e)}rotateY(e){return this.rotateOnAxis(cu,e)}rotateZ(e){return this.rotateOnAxis(hu,e)}translateOnAxis(e,t){return au.copy(e).applyQuaternion(this.quaternion),this.position.add(au.multiplyScalar(t)),this}translateX(e){return this.translateOnAxis(lu,e)}translateY(e){return this.translateOnAxis(cu,e)}translateZ(e){return this.translateOnAxis(hu,e)}localToWorld(e){return this.updateWorldMatrix(!0,!1),e.applyMatrix4(this.matrixWorld)}worldToLocal(e){return this.updateWorldMatrix(!0,!1),e.applyMatrix4($n.copy(this.matrixWorld).invert())}lookAt(e,t,n){e.isVector3?So.copy(e):So.set(e,t,n);let i=this.parent;this.updateWorldMatrix(!0,!1),nr.setFromMatrixPosition(this.matrixWorld),this.isCamera||this.isLight?$n.lookAt(nr,So,this.up):$n.lookAt(So,nr,this.up),this.quaternion.setFromRotationMatrix($n),i&&($n.extractRotation(i.matrixWorld),cs.setFromRotationMatrix($n),this.quaternion.premultiply(cs.invert()))}add(e){if(arguments.length>1){for(let t=0;t<arguments.length;t++)this.add(arguments[t]);return this}return e===this?(Ye("Object3D.add: object can't be added as a child of itself.",e),this):(e&&e.isObject3D?(e.removeFromParent(),e.parent=this,this.children.push(e),e.dispatchEvent(uu),hs.child=e,this.dispatchEvent(hs),hs.child=null):Ye("Object3D.add: object not an instance of THREE.Object3D.",e),this)}remove(e){if(arguments.length>1){for(let n=0;n<arguments.length;n++)this.remove(arguments[n]);return this}let t=this.children.indexOf(e);return t!==-1&&(e.parent=null,this.children.splice(t,1),e.dispatchEvent(Ep),kl.child=e,this.dispatchEvent(kl),kl.child=null),this}removeFromParent(){let e=this.parent;return e!==null&&e.remove(this),this}clear(){return this.remove(...this.children)}attach(e){return this.updateWorldMatrix(!0,!1),$n.copy(this.matrixWorld).invert(),e.parent!==null&&(e.parent.updateWorldMatrix(!0,!1),$n.multiply(e.parent.matrixWorld)),e.applyMatrix4($n),e.removeFromParent(),e.parent=this,this.children.push(e),e.updateWorldMatrix(!1,!0),e.dispatchEvent(uu),hs.child=e,this.dispatchEvent(hs),hs.child=null,this}getObjectById(e){return this.getObjectByProperty("id",e)}getObjectByName(e){return this.getObjectByProperty("name",e)}getObjectByProperty(e,t){if(this[e]===t)return this;for(let n=0,i=this.children.length;n<i;n++){let o=this.children[n].getObjectByProperty(e,t);if(o!==void 0)return o}}getObjectsByProperty(e,t,n=[]){this[e]===t&&n.push(this);let i=this.children;for(let r=0,o=i.length;r<o;r++)i[r].getObjectsByProperty(e,t,n);return n}getWorldPosition(e){return this.updateWorldMatrix(!0,!1),e.setFromMatrixPosition(this.matrixWorld)}getWorldQuaternion(e){return this.updateWorldMatrix(!0,!1),this.matrixWorld.decompose(nr,e,Ap),e}getWorldScale(e){return this.updateWorldMatrix(!0,!1),this.matrixWorld.decompose(nr,Tp,e),e}getWorldDirection(e){this.updateWorldMatrix(!0,!1);let t=this.matrixWorld.elements;return e.set(t[8],t[9],t[10]).normalize()}raycast(){}intersectsFrustum(){return!0}traverse(e){e(this);let t=this.children;for(let n=0,i=t.length;n<i;n++)t[n].traverse(e)}traverseVisible(e){if(this.visible===!1)return;e(this);let t=this.children;for(let n=0,i=t.length;n<i;n++)t[n].traverseVisible(e)}traverseAncestors(e){let t=this.parent;t!==null&&(e(t),t.traverseAncestors(e))}updateMatrix(){this.matrix.compose(this.position,this.quaternion,this.scale);let e=this.pivot;if(e!==null){let t=e.x,n=e.y,i=e.z,r=this.matrix.elements;r[12]+=t-r[0]*t-r[4]*n-r[8]*i,r[13]+=n-r[1]*t-r[5]*n-r[9]*i,r[14]+=i-r[2]*t-r[6]*n-r[10]*i}this.matrixWorldNeedsUpdate=!0}updateMatrixWorld(e){this.matrixAutoUpdate&&this.updateMatrix(),(this.matrixWorldNeedsUpdate||e)&&(this.matrixWorldAutoUpdate===!0&&(this.parent===null?this.matrixWorld.copy(this.matrix):this.matrixWorld.multiplyMatrices(this.parent.matrixWorld,this.matrix)),this.matrixWorldNeedsUpdate=!1,e=!0);let t=this.children;for(let n=0,i=t.length;n<i;n++)t[n].updateMatrixWorld(e)}updateWorldMatrix(e,t,n=!1){let i=this.parent;if(e===!0&&i!==null&&i.updateWorldMatrix(!0,!1),this.matrixAutoUpdate&&this.updateMatrix(),(this.matrixWorldNeedsUpdate||n)&&(this.matrixWorldAutoUpdate===!0&&(this.parent===null?this.matrixWorld.copy(this.matrix):this.matrixWorld.multiplyMatrices(this.parent.matrixWorld,this.matrix)),this.matrixWorldNeedsUpdate=!1,n=!0),t===!0){let r=this.children;for(let o=0,a=r.length;o<a;o++)r[o].updateWorldMatrix(!1,!0,n)}}toJSON(e){let t=e===void 0||typeof e=="string",n={};t&&(e={geometries:{},materials:{},textures:{},images:{},shapes:{},skeletons:{},animations:{},nodes:{}},n.metadata={version:4.7,type:"Object",generator:"Object3D.toJSON"});let i={};i.uuid=this.uuid,i.type=this.type,i.name=this.name,i.castShadow=this.castShadow,i.receiveShadow=this.receiveShadow,i.visible=this.visible,i.frustumCulled=this.frustumCulled,i.renderOrder=this.renderOrder,i.static=this.static,i.matrixAutoUpdate=this.matrixAutoUpdate,Object.keys(this.userData).length>0&&(i.userData=this.userData),i.layers=this.layers.mask,i.matrix=this.matrix.toArray(),i.up=this.up.toArray(),this.pivot!==null&&(i.pivot=this.pivot.toArray()),this.morphTargetDictionary!==void 0&&(i.morphTargetDictionary=Object.assign({},this.morphTargetDictionary)),this.morphTargetInfluences!==void 0&&(i.morphTargetInfluences=this.morphTargetInfluences.slice()),this.isInstancedMesh&&(i.type="InstancedMesh",i.count=this.count,i.instanceMatrix=this.instanceMatrix.toJSON(),this.instanceColor!==null&&(i.instanceColor=this.instanceColor.toJSON())),this.isBatchedMesh&&(i.type="BatchedMesh",i.perObjectFrustumCulled=this.perObjectFrustumCulled,i.sortObjects=this.sortObjects,i.drawRanges=this._drawRanges,i.reservedRanges=this._reservedRanges,i.geometryInfo=this._geometryInfo.map(a=>({...a,boundingBox:a.boundingBox?a.boundingBox.toJSON():void 0,boundingSphere:a.boundingSphere?a.boundingSphere.toJSON():void 0})),i.instanceInfo=this._instanceInfo.map(a=>({...a})),i.availableInstanceIds=this._availableInstanceIds.slice(),i.availableGeometryIds=this._availableGeometryIds.slice(),i.nextIndexStart=this._nextIndexStart,i.nextVertexStart=this._nextVertexStart,i.geometryCount=this._geometryCount,i.maxInstanceCount=this._maxInstanceCount,i.maxVertexCount=this._maxVertexCount,i.maxIndexCount=this._maxIndexCount,i.geometryInitialized=this._geometryInitialized,i.matricesTexture=this._matricesTexture.toJSON(e),i.indirectTexture=this._indirectTexture.toJSON(e),this._colorsTexture!==null&&(i.colorsTexture=this._colorsTexture.toJSON(e)),this.boundingSphere!==null&&(i.boundingSphere=this.boundingSphere.toJSON()),this.boundingBox!==null&&(i.boundingBox=this.boundingBox.toJSON()));function r(a,l){return a[l.uuid]===void 0&&(a[l.uuid]=l.toJSON(e)),l.uuid}if(this.isScene)this.background&&(this.background.isColor?i.background=this.background.toJSON():this.background.isTexture&&(i.background=this.background.toJSON(e).uuid)),this.environment&&this.environment.isTexture&&this.environment.isRenderTargetTexture!==!0&&(i.environment=this.environment.toJSON(e).uuid);else if(this.isMesh||this.isLine||this.isPoints){i.geometry=r(e.geometries,this.geometry);let a=this.geometry.parameters;if(a!==void 0&&a.shapes!==void 0){let l=a.shapes;if(Array.isArray(l))for(let c=0,u=l.length;c<u;c++){let h=l[c];r(e.shapes,h)}else r(e.shapes,l)}}if(this.isSkinnedMesh&&(i.bindMode=this.bindMode,i.bindMatrix=this.bindMatrix.toArray(),this.skeleton!==void 0&&(r(e.skeletons,this.skeleton),i.skeleton=this.skeleton.uuid)),this.material!==void 0)if(Array.isArray(this.material)){let a=[];for(let l=0,c=this.material.length;l<c;l++)a.push(r(e.materials,this.material[l]));i.material=a}else i.material=r(e.materials,this.material);if(this.children.length>0){i.children=[];for(let a=0;a<this.children.length;a++)i.children.push(this.children[a].toJSON(e).object)}if(this.animations.length>0){i.animations=[];for(let a=0;a<this.animations.length;a++){let l=this.animations[a];i.animations.push(r(e.animations,l))}}if(t){let a=o(e.geometries),l=o(e.materials),c=o(e.textures),u=o(e.images),h=o(e.shapes),d=o(e.skeletons),f=o(e.animations),m=o(e.nodes);a.length>0&&(n.geometries=a),l.length>0&&(n.materials=l),c.length>0&&(n.textures=c),u.length>0&&(n.images=u),h.length>0&&(n.shapes=h),d.length>0&&(n.skeletons=d),f.length>0&&(n.animations=f),m.length>0&&(n.nodes=m)}return n.object=i,n;function o(a){let l=[];for(let c in a){let u=a[c];delete u.metadata,l.push(u)}return l}}clone(e){return new this.constructor().copy(this,e)}copy(e,t=!0){if(this.name=e.name,this.up.copy(e.up),this.position.copy(e.position),this.rotation.order=e.rotation.order,this.quaternion.copy(e.quaternion),this.scale.copy(e.scale),this.pivot=e.pivot!==null?e.pivot.clone():null,this.matrix.copy(e.matrix),this.matrixWorld.copy(e.matrixWorld),this.matrixAutoUpdate=e.matrixAutoUpdate,this.matrixWorldAutoUpdate=e.matrixWorldAutoUpdate,this.matrixWorldNeedsUpdate=e.matrixWorldNeedsUpdate,this.layers.mask=e.layers.mask,this.visible=e.visible,this.castShadow=e.castShadow,this.receiveShadow=e.receiveShadow,this.frustumCulled=e.frustumCulled,this.renderOrder=e.renderOrder,this.static=e.static,this.animations=e.animations.slice(),this.userData=JSON.parse(JSON.stringify(e.userData)),t===!0)for(let n=0;n<e.children.length;n++){let i=e.children[n];this.add(i.clone())}return this}dispose(){this.dispatchEvent({type:"dispose"})}};At.DEFAULT_UP=new L(0,1,0);At.DEFAULT_MATRIX_AUTO_UPDATE=!0;At.DEFAULT_MATRIX_WORLD_AUTO_UPDATE=!0;var ut=class extends At{constructor(){super(),this.isGroup=!0,this.type="Group"}},Rp={type:"move"},Ps=class{constructor(){this._targetRay=null,this._grip=null,this._hand=null}getHandSpace(){return this._hand===null&&(this._hand=new ut,this._hand.matrixAutoUpdate=!1,this._hand.visible=!1,this._hand.joints={},this._hand.inputState={pinching:!1}),this._hand}getTargetRaySpace(){return this._targetRay===null&&(this._targetRay=new ut,this._targetRay.matrixAutoUpdate=!1,this._targetRay.visible=!1,this._targetRay.hasLinearVelocity=!1,this._targetRay.linearVelocity=new L,this._targetRay.hasAngularVelocity=!1,this._targetRay.angularVelocity=new L),this._targetRay}getGripSpace(){return this._grip===null&&(this._grip=new ut,this._grip.matrixAutoUpdate=!1,this._grip.visible=!1,this._grip.hasLinearVelocity=!1,this._grip.linearVelocity=new L,this._grip.hasAngularVelocity=!1,this._grip.angularVelocity=new L,this._grip.eventsEnabled=!1),this._grip}dispatchEvent(e){return this._targetRay!==null&&this._targetRay.dispatchEvent(e),this._grip!==null&&this._grip.dispatchEvent(e),this._hand!==null&&this._hand.dispatchEvent(e),this}connect(e){if(e&&e.hand){let t=this._hand;if(t)for(let n of e.hand.values())this._getHandJoint(t,n)}return this.dispatchEvent({type:"connected",data:e}),this}disconnect(e){return this.dispatchEvent({type:"disconnected",data:e}),this._targetRay!==null&&(this._targetRay.visible=!1),this._grip!==null&&(this._grip.visible=!1),this._hand!==null&&(this._hand.visible=!1),this}update(e,t,n){let i=null,r=null,o=null,a=this._targetRay,l=this._grip,c=this._hand;if(e&&t.session.visibilityState!=="visible-blurred"){if(c&&e.hand){o=!0;for(let _ of e.hand.values()){let p=t.getJointPose(_,n),g=this._getHandJoint(c,_);p!==null&&(g.matrix.fromArray(p.transform.matrix),g.matrix.decompose(g.position,g.quaternion,g.scale),g.matrixWorldNeedsUpdate=!0,g.jointRadius=p.radius),g.visible=p!==null}let u=c.joints["index-finger-tip"],h=c.joints["thumb-tip"],d=u.position.distanceTo(h.position),f=.02,m=.005;c.inputState.pinching&&d>f+m?(c.inputState.pinching=!1,this.dispatchEvent({type:"pinchend",handedness:e.handedness,target:this})):!c.inputState.pinching&&d<=f-m&&(c.inputState.pinching=!0,this.dispatchEvent({type:"pinchstart",handedness:e.handedness,target:this}))}else l!==null&&e.gripSpace&&(r=t.getPose(e.gripSpace,n),r!==null&&(l.matrix.fromArray(r.transform.matrix),l.matrix.decompose(l.position,l.quaternion,l.scale),l.matrixWorldNeedsUpdate=!0,r.linearVelocity?(l.hasLinearVelocity=!0,l.linearVelocity.copy(r.linearVelocity)):l.hasLinearVelocity=!1,r.angularVelocity?(l.hasAngularVelocity=!0,l.angularVelocity.copy(r.angularVelocity)):l.hasAngularVelocity=!1,l.eventsEnabled&&l.dispatchEvent({type:"gripUpdated",data:e,target:this})));a!==null&&(i=t.getPose(e.targetRaySpace,n),i===null&&r!==null&&(i=r),i!==null&&(a.matrix.fromArray(i.transform.matrix),a.matrix.decompose(a.position,a.quaternion,a.scale),a.matrixWorldNeedsUpdate=!0,i.linearVelocity?(a.hasLinearVelocity=!0,a.linearVelocity.copy(i.linearVelocity)):a.hasLinearVelocity=!1,i.angularVelocity?(a.hasAngularVelocity=!0,a.angularVelocity.copy(i.angularVelocity)):a.hasAngularVelocity=!1,this.dispatchEvent(Rp)))}return a!==null&&(a.visible=i!==null),l!==null&&(l.visible=r!==null),c!==null&&(c.visible=o!==null),this}_getHandJoint(e,t){if(e.joints[t.jointName]===void 0){let n=new ut;n.matrixAutoUpdate=!1,n.visible=!1,e.joints[t.jointName]=n,e.add(n)}return e.joints[t.jointName]}},Rf={aliceblue:15792383,antiquewhite:16444375,aqua:65535,aquamarine:8388564,azure:15794175,beige:16119260,bisque:16770244,black:0,blanchedalmond:16772045,blue:255,blueviolet:9055202,brown:10824234,burlywood:14596231,cadetblue:6266528,chartreuse:8388352,chocolate:13789470,coral:16744272,cornflowerblue:6591981,cornsilk:16775388,crimson:14423100,cyan:65535,darkblue:139,darkcyan:35723,darkgoldenrod:12092939,darkgray:11119017,darkgreen:25600,darkgrey:11119017,darkkhaki:12433259,darkmagenta:9109643,darkolivegreen:5597999,darkorange:16747520,darkorchid:10040012,darkred:9109504,darksalmon:15308410,darkseagreen:9419919,darkslateblue:4734347,darkslategray:3100495,darkslategrey:3100495,darkturquoise:52945,darkviolet:9699539,deeppink:16716947,deepskyblue:49151,dimgray:6908265,dimgrey:6908265,dodgerblue:2003199,firebrick:11674146,floralwhite:16775920,forestgreen:2263842,fuchsia:16711935,gainsboro:14474460,ghostwhite:16316671,gold:16766720,goldenrod:14329120,gray:8421504,green:32768,greenyellow:11403055,grey:8421504,honeydew:15794160,hotpink:16738740,indianred:13458524,indigo:4915330,ivory:16777200,khaki:15787660,lavender:15132410,lavenderblush:16773365,lawngreen:8190976,lemonchiffon:16775885,lightblue:11393254,lightcoral:15761536,lightcyan:14745599,lightgoldenrodyellow:16448210,lightgray:13882323,lightgreen:9498256,lightgrey:13882323,lightpink:16758465,lightsalmon:16752762,lightseagreen:2142890,lightskyblue:8900346,lightslategray:7833753,lightslategrey:7833753,lightsteelblue:11584734,lightyellow:16777184,lime:65280,limegreen:3329330,linen:16445670,magenta:16711935,maroon:8388608,mediumaquamarine:6737322,mediumblue:205,mediumorchid:12211667,mediumpurple:9662683,mediumseagreen:3978097,mediumslateblue:8087790,mediumspringgreen:64154,mediumturquoise:4772300,mediumvioletred:13047173,midnightblue:1644912,mintcream:16121850,mistyrose:16770273,moccasin:16770229,navajowhite:16768685,navy:128,oldlace:16643558,olive:8421376,olivedrab:7048739,orange:16753920,orangered:16729344,orchid:14315734,palegoldenrod:15657130,palegreen:10025880,paleturquoise:11529966,palevioletred:14381203,papayawhip:16773077,peachpuff:16767673,peru:13468991,pink:16761035,plum:14524637,powderblue:11591910,purple:8388736,rebeccapurple:6697881,red:16711680,rosybrown:12357519,royalblue:4286945,saddlebrown:9127187,salmon:16416882,sandybrown:16032864,seagreen:3050327,seashell:16774638,sienna:10506797,silver:12632256,skyblue:8900331,slateblue:6970061,slategray:7372944,slategrey:7372944,snow:16775930,springgreen:65407,steelblue:4620980,tan:13808780,teal:32896,thistle:14204888,tomato:16737095,turquoise:4251856,violet:15631086,wheat:16113331,white:16777215,whitesmoke:16119285,yellow:16776960,yellowgreen:10145074},yi={h:0,s:0,l:0},Mo={h:0,s:0,l:0};function Vl(s,e,t){return t<0&&(t+=1),t>1&&(t-=1),t<1/6?s+(e-s)*6*t:t<1/2?e:t<2/3?s+(e-s)*6*(2/3-t):s}var Ke=class{constructor(e,t,n){return this.isColor=!0,this.r=1,this.g=1,this.b=1,this.set(e,t,n)}set(e,t,n){if(t===void 0&&n===void 0){let i=e;i&&i.isColor?this.copy(i):typeof i=="number"?this.setHex(i):typeof i=="string"&&this.setStyle(i)}else this.setRGB(e,t,n);return this}setScalar(e){return this.r=e,this.g=e,this.b=e,this}setHex(e,t=wt){return e=Math.floor(e),this.r=(e>>16&255)/255,this.g=(e>>8&255)/255,this.b=(e&255)/255,lt.colorSpaceToWorking(this,t),this}setRGB(e,t,n,i=lt.workingColorSpace){return this.r=e,this.g=t,this.b=n,lt.colorSpaceToWorking(this,i),this}setHSL(e,t,n,i=lt.workingColorSpace){if(e=gp(e,1),t=it(t,0,1),n=it(n,0,1),t===0)this.r=this.g=this.b=n;else{let r=n<=.5?n*(1+t):n+t-n*t,o=2*n-r;this.r=Vl(o,r,e+1/3),this.g=Vl(o,r,e),this.b=Vl(o,r,e-1/3)}return lt.colorSpaceToWorking(this,i),this}setStyle(e,t=wt){function n(r){r!==void 0&&parseFloat(r)<1&&Xe("Color: Alpha component of "+e+" will be ignored.")}let i;if(i=/^(\w+)\(([^\)]*)\)/.exec(e)){let r,o=i[1],a=i[2];switch(o){case"rgb":case"rgba":if(r=/^\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*(\d+(?:\.\d+)?|\.\d+)\s*)?$/.exec(a))return n(r[4]),this.setRGB(Math.min(255,parseInt(r[1],10))/255,Math.min(255,parseInt(r[2],10))/255,Math.min(255,parseInt(r[3],10))/255,t);if(r=/^\s*(\d+(?:\.\d+)?|\.\d+)\%\s*,\s*(\d+(?:\.\d+)?|\.\d+)\%\s*,\s*(\d+(?:\.\d+)?|\.\d+)\%\s*(?:,\s*(\d+(?:\.\d+)?|\.\d+)\s*)?$/.exec(a))return n(r[4]),this.setRGB(Math.min(100,parseFloat(r[1]))/100,Math.min(100,parseFloat(r[2]))/100,Math.min(100,parseFloat(r[3]))/100,t);break;case"hsl":case"hsla":if(r=/^\s*([+-]?(?:\d+(?:\.\d+)?|\.\d+))\s*,\s*(\d+(?:\.\d+)?|\.\d+)\%\s*,\s*(\d+(?:\.\d+)?|\.\d+)\%\s*(?:,\s*(\d+(?:\.\d+)?|\.\d+)\s*)?$/.exec(a))return n(r[4]),this.setHSL(parseFloat(r[1])/360,parseFloat(r[2])/100,parseFloat(r[3])/100,t);break;default:Xe("Color: Unknown color model "+e)}}else if(i=/^\#([A-Fa-f\d]+)$/.exec(e)){let r=i[1],o=r.length;if(o===3)return this.setRGB(parseInt(r.charAt(0),16)/15,parseInt(r.charAt(1),16)/15,parseInt(r.charAt(2),16)/15,t);if(o===6)return this.setHex(parseInt(r,16),t);Xe("Color: Invalid hex color "+e)}else if(e&&e.length>0)return this.setColorName(e,t);return this}setColorName(e,t=wt){let n=Rf[e.toLowerCase()];return n!==void 0?this.setHex(n,t):Xe("Color: Unknown color "+e),this}clone(){return new this.constructor(this.r,this.g,this.b)}copy(e){return this.r=e.r,this.g=e.g,this.b=e.b,this}copySRGBToLinear(e){return this.r=ni(e.r),this.g=ni(e.g),this.b=ni(e.b),this}copyLinearToSRGB(e){return this.r=Ss(e.r),this.g=Ss(e.g),this.b=Ss(e.b),this}convertSRGBToLinear(){return this.copySRGBToLinear(this),this}convertLinearToSRGB(){return this.copyLinearToSRGB(this),this}getHex(e=wt){return lt.workingToColorSpace(Kt.copy(this),e),Math.round(it(Kt.r*255,0,255))*65536+Math.round(it(Kt.g*255,0,255))*256+Math.round(it(Kt.b*255,0,255))}getHexString(e=wt){return("000000"+this.getHex(e).toString(16)).slice(-6)}getHSL(e,t=lt.workingColorSpace){lt.workingToColorSpace(Kt.copy(this),t);let n=Kt.r,i=Kt.g,r=Kt.b,o=Math.max(n,i,r),a=Math.min(n,i,r),l,c,u=(a+o)/2;if(a===o)l=0,c=0;else{let h=o-a;switch(c=u<=.5?h/(o+a):h/(2-o-a),o){case n:l=(i-r)/h+(i<r?6:0);break;case i:l=(r-n)/h+2;break;case r:l=(n-i)/h+4;break}l/=6}return e.h=l,e.s=c,e.l=u,e}getRGB(e,t=lt.workingColorSpace){return lt.workingToColorSpace(Kt.copy(this),t),e.r=Kt.r,e.g=Kt.g,e.b=Kt.b,e}getStyle(e=wt){lt.workingToColorSpace(Kt.copy(this),e);let t=Kt.r,n=Kt.g,i=Kt.b;return e!==wt?`color(${e} ${t.toFixed(3)} ${n.toFixed(3)} ${i.toFixed(3)})`:`rgb(${Math.round(t*255)},${Math.round(n*255)},${Math.round(i*255)})`}offsetHSL(e,t,n){return this.getHSL(yi),this.setHSL(yi.h+e,yi.s+t,yi.l+n)}add(e){return this.r+=e.r,this.g+=e.g,this.b+=e.b,this}addColors(e,t){return this.r=e.r+t.r,this.g=e.g+t.g,this.b=e.b+t.b,this}addScalar(e){return this.r+=e,this.g+=e,this.b+=e,this}sub(e){return this.r=Math.max(0,this.r-e.r),this.g=Math.max(0,this.g-e.g),this.b=Math.max(0,this.b-e.b),this}multiply(e){return this.r*=e.r,this.g*=e.g,this.b*=e.b,this}multiplyScalar(e){return this.r*=e,this.g*=e,this.b*=e,this}lerp(e,t){return this.r+=(e.r-this.r)*t,this.g+=(e.g-this.g)*t,this.b+=(e.b-this.b)*t,this}lerpColors(e,t,n){return this.r=e.r+(t.r-e.r)*n,this.g=e.g+(t.g-e.g)*n,this.b=e.b+(t.b-e.b)*n,this}lerpHSL(e,t){this.getHSL(yi),e.getHSL(Mo);let n=Ul(yi.h,Mo.h,t),i=Ul(yi.s,Mo.s,t),r=Ul(yi.l,Mo.l,t);return this.setHSL(n,i,r),this}setFromVector3(e){return this.r=e.x,this.g=e.y,this.b=e.z,this}applyMatrix3(e){let t=this.r,n=this.g,i=this.b,r=e.elements;return this.r=r[0]*t+r[3]*n+r[6]*i,this.g=r[1]*t+r[4]*n+r[7]*i,this.b=r[2]*t+r[5]*n+r[8]*i,this}equals(e){return e.r===this.r&&e.g===this.g&&e.b===this.b}fromArray(e,t=0){return this.r=e[t],this.g=e[t+1],this.b=e[t+2],this}toArray(e=[],t=0){return e[t]=this.r,e[t+1]=this.g,e[t+2]=this.b,e}fromBufferAttribute(e,t){return this.r=e.getX(t),this.g=e.getY(t),this.b=e.getZ(t),this}toJSON(){return this.getHex()}*[Symbol.iterator](){yield this.r,yield this.g,yield this.b}},Kt=new Ke;Ke.NAMES=Rf;var si=class extends At{constructor(){super(),this.isScene=!0,this.type="Scene",this.background=null,this.environment=null,this.fog=null,this.backgroundBlurriness=0,this.backgroundIntensity=1,this.backgroundRotation=new Ut,this.environmentIntensity=1,this.environmentRotation=new Ut,this.overrideMaterial=null,typeof __THREE_DEVTOOLS__<"u"&&__THREE_DEVTOOLS__.dispatchEvent(new CustomEvent("observe",{detail:this}))}copy(e,t){return super.copy(e,t),this.background=e.background!==null?e.background.clone():null,this.environment=e.environment!==null?e.environment.clone():null,this.fog=e.fog!==null?e.fog.clone():null,this.backgroundBlurriness=e.backgroundBlurriness,this.backgroundIntensity=e.backgroundIntensity,this.backgroundRotation.copy(e.backgroundRotation),this.environmentIntensity=e.environmentIntensity,this.environmentRotation.copy(e.environmentRotation),this.overrideMaterial=e.overrideMaterial!==null?e.overrideMaterial.clone():null,this.matrixAutoUpdate=e.matrixAutoUpdate,this}toJSON(e){let t=super.toJSON(e);return this.fog!==null&&(t.object.fog=this.fog.toJSON()),t.object.backgroundBlurriness=this.backgroundBlurriness,t.object.backgroundIntensity=this.backgroundIntensity,t.object.backgroundRotation=this.backgroundRotation.toArray(),t.object.environmentIntensity=this.environmentIntensity,t.object.environmentRotation=this.environmentRotation.toArray(),t}},Tn=new L,Kn=new L,Gl=new L,jn=new L,us=new L,fs=new L,fu=new L,Hl=new L,Wl=new L,Xl=new L,ql=new _t,Yl=new _t,Zl=new _t,On=class s{constructor(e=new L,t=new L,n=new L){this.a=e,this.b=t,this.c=n}static getNormal(e,t,n,i){i.subVectors(n,t),Tn.subVectors(e,t),i.cross(Tn);let r=i.lengthSq();return r>0?i.multiplyScalar(1/Math.sqrt(r)):i.set(0,0,0)}static getBarycoord(e,t,n,i,r){Tn.subVectors(i,t),Kn.subVectors(n,t),Gl.subVectors(e,t);let o=Tn.dot(Tn),a=Tn.dot(Kn),l=Tn.dot(Gl),c=Kn.dot(Kn),u=Kn.dot(Gl),h=o*c-a*a;if(h===0)return r.set(0,0,0),null;let d=1/h,f=(c*l-a*u)*d,m=(o*u-a*l)*d;return r.set(1-f-m,m,f)}static containsPoint(e,t,n,i){return this.getBarycoord(e,t,n,i,jn)===null?!1:jn.x>=0&&jn.y>=0&&jn.x+jn.y<=1}static getInterpolation(e,t,n,i,r,o,a,l){return this.getBarycoord(e,t,n,i,jn)===null?(l.x=0,l.y=0,"z"in l&&(l.z=0),"w"in l&&(l.w=0),null):(l.setScalar(0),l.addScaledVector(r,jn.x),l.addScaledVector(o,jn.y),l.addScaledVector(a,jn.z),l)}static getInterpolatedAttribute(e,t,n,i,r,o){return ql.setScalar(0),Yl.setScalar(0),Zl.setScalar(0),ql.fromBufferAttribute(e,t),Yl.fromBufferAttribute(e,n),Zl.fromBufferAttribute(e,i),o.setScalar(0),o.addScaledVector(ql,r.x),o.addScaledVector(Yl,r.y),o.addScaledVector(Zl,r.z),o}static isFrontFacing(e,t,n,i){return Tn.subVectors(n,t),Kn.subVectors(e,t),Tn.cross(Kn).dot(i)<0}set(e,t,n){return this.a.copy(e),this.b.copy(t),this.c.copy(n),this}setFromPointsAndIndices(e,t,n,i){return this.a.copy(e[t]),this.b.copy(e[n]),this.c.copy(e[i]),this}setFromAttributeAndIndices(e,t,n,i){return this.a.fromBufferAttribute(e,t),this.b.fromBufferAttribute(e,n),this.c.fromBufferAttribute(e,i),this}clone(){return new this.constructor().copy(this)}copy(e){return this.a.copy(e.a),this.b.copy(e.b),this.c.copy(e.c),this}getArea(){return Tn.subVectors(this.c,this.b),Kn.subVectors(this.a,this.b),Tn.cross(Kn).length()*.5}getMidpoint(e){return e.addVectors(this.a,this.b).add(this.c).multiplyScalar(1/3)}getNormal(e){return s.getNormal(this.a,this.b,this.c,e)}getPlane(e){return e.setFromCoplanarPoints(this.a,this.b,this.c)}getBarycoord(e,t){return s.getBarycoord(e,this.a,this.b,this.c,t)}getInterpolation(e,t,n,i,r){return s.getInterpolation(e,this.a,this.b,this.c,t,n,i,r)}containsPoint(e){return s.containsPoint(e,this.a,this.b,this.c)}isFrontFacing(e){return s.isFrontFacing(this.a,this.b,this.c,e)}intersectsBox(e){return e.intersectsTriangle(this)}closestPointToPoint(e,t){let n=this.a,i=this.b,r=this.c,o,a;us.subVectors(i,n),fs.subVectors(r,n),Hl.subVectors(e,n);let l=us.dot(Hl),c=fs.dot(Hl);if(l<=0&&c<=0)return t.copy(n);Wl.subVectors(e,i);let u=us.dot(Wl),h=fs.dot(Wl);if(u>=0&&h<=u)return t.copy(i);let d=l*h-u*c;if(d<=0&&l>=0&&u<=0&&l-u>0)return o=l/(l-u),t.copy(n).addScaledVector(us,o);Xl.subVectors(e,r);let f=us.dot(Xl),m=fs.dot(Xl);if(m>=0&&f<=m)return t.copy(r);let _=f*c-l*m;if(_<=0&&c>=0&&m<=0)return a=c/(c-m),t.copy(n).addScaledVector(fs,a);let p=u*m-f*h;if(p<=0&&h-u>=0&&f-m>=0)return fu.subVectors(r,i),a=(h-u)/(h-u+(f-m)),t.copy(i).addScaledVector(fu,a);let g=1/(p+_+d);return o=_*g,a=d*g,t.copy(n).addScaledVector(us,o).addScaledVector(fs,a)}equals(e){return e.a.equals(this.a)&&e.b.equals(this.b)&&e.c.equals(this.c)}},Mt=class{constructor(e=new L(1/0,1/0,1/0),t=new L(-1/0,-1/0,-1/0)){this.isBox3=!0,this.min=e,this.max=t}set(e,t){return this.min.copy(e),this.max.copy(t),this}setFromArray(e){this.makeEmpty();for(let t=0,n=e.length;t<n;t+=3)this.expandByPoint(En.fromArray(e,t));return this}setFromBufferAttribute(e){this.makeEmpty();for(let t=0,n=e.count;t<n;t++)this.expandByPoint(En.fromBufferAttribute(e,t));return this}setFromPoints(e){this.makeEmpty();for(let t=0,n=e.length;t<n;t++)this.expandByPoint(e[t]);return this}setFromCenterAndSize(e,t){let n=En.copy(t).multiplyScalar(.5);return this.min.copy(e).sub(n),this.max.copy(e).add(n),this}setFromObject(e,t=!1){return this.makeEmpty(),this.expandByObject(e,t)}clone(){return new this.constructor().copy(this)}copy(e){return this.min.copy(e.min),this.max.copy(e.max),this}makeEmpty(){return this.min.x=this.min.y=this.min.z=1/0,this.max.x=this.max.y=this.max.z=-1/0,this}isEmpty(){return this.max.x<this.min.x||this.max.y<this.min.y||this.max.z<this.min.z}getCenter(e){return this.isEmpty()?e.set(0,0,0):e.addVectors(this.min,this.max).multiplyScalar(.5)}getSize(e){return this.isEmpty()?e.set(0,0,0):e.subVectors(this.max,this.min)}expandByPoint(e){return this.min.min(e),this.max.max(e),this}expandByVector(e){return this.min.sub(e),this.max.add(e),this}expandByScalar(e){return this.min.addScalar(-e),this.max.addScalar(e),this}expandByObject(e,t=!1){e.updateWorldMatrix(!1,!1);let n=e.geometry;if(n!==void 0){let r=n.getAttribute("position");if(t===!0&&r!==void 0&&e.isInstancedMesh!==!0&&e.isBatchedMesh!==!0)for(let o=0,a=r.count;o<a;o++)e.isMesh===!0?e.getVertexPosition(o,En):En.fromBufferAttribute(r,o),En.applyMatrix4(e.matrixWorld),this.expandByPoint(En);else e.boundingBox!==void 0?(e.boundingBox===null&&e.computeBoundingBox(),bo.copy(e.boundingBox)):(n.boundingBox===null&&n.computeBoundingBox(),bo.copy(n.boundingBox)),bo.applyMatrix4(e.matrixWorld),this.union(bo)}let i=e.children;for(let r=0,o=i.length;r<o;r++)this.expandByObject(i[r],t);return this}containsPoint(e){return e.x>=this.min.x&&e.x<=this.max.x&&e.y>=this.min.y&&e.y<=this.max.y&&e.z>=this.min.z&&e.z<=this.max.z}containsBox(e){return this.min.x<=e.min.x&&e.max.x<=this.max.x&&this.min.y<=e.min.y&&e.max.y<=this.max.y&&this.min.z<=e.min.z&&e.max.z<=this.max.z}getParameter(e,t){return t.set((e.x-this.min.x)/(this.max.x-this.min.x),(e.y-this.min.y)/(this.max.y-this.min.y),(e.z-this.min.z)/(this.max.z-this.min.z))}intersectsBox(e){return e.max.x>=this.min.x&&e.min.x<=this.max.x&&e.max.y>=this.min.y&&e.min.y<=this.max.y&&e.max.z>=this.min.z&&e.min.z<=this.max.z}intersectsSphere(e){return this.clampPoint(e.center,En),En.distanceToSquared(e.center)<=e.radius*e.radius}intersectsPlane(e){let t,n;return e.normal.x>0?(t=e.normal.x*this.min.x,n=e.normal.x*this.max.x):(t=e.normal.x*this.max.x,n=e.normal.x*this.min.x),e.normal.y>0?(t+=e.normal.y*this.min.y,n+=e.normal.y*this.max.y):(t+=e.normal.y*this.max.y,n+=e.normal.y*this.min.y),e.normal.z>0?(t+=e.normal.z*this.min.z,n+=e.normal.z*this.max.z):(t+=e.normal.z*this.max.z,n+=e.normal.z*this.min.z),t<=-e.constant&&n>=-e.constant}intersectsTriangle(e){if(this.isEmpty())return!1;this.getCenter(ir),wo.subVectors(this.max,ir),ds.subVectors(e.a,ir),ps.subVectors(e.b,ir),ms.subVectors(e.c,ir),vi.subVectors(ps,ds),Si.subVectors(ms,ps),Oi.subVectors(ds,ms);let t=[0,-vi.z,vi.y,0,-Si.z,Si.y,0,-Oi.z,Oi.y,vi.z,0,-vi.x,Si.z,0,-Si.x,Oi.z,0,-Oi.x,-vi.y,vi.x,0,-Si.y,Si.x,0,-Oi.y,Oi.x,0];return!Jl(t,ds,ps,ms,wo)||(t=[1,0,0,0,1,0,0,0,1],!Jl(t,ds,ps,ms,wo))?!1:(Ao.crossVectors(vi,Si),t=[Ao.x,Ao.y,Ao.z],Jl(t,ds,ps,ms,wo))}clampPoint(e,t){return t.copy(e).clamp(this.min,this.max)}distanceToPoint(e){return this.clampPoint(e,En).distanceTo(e)}getBoundingSphere(e){return this.isEmpty()?e.makeEmpty():(this.getCenter(e.center),e.radius=this.getSize(En).length()*.5),e}intersect(e){return this.min.max(e.min),this.max.min(e.max),this.isEmpty()&&this.makeEmpty(),this}union(e){return this.min.min(e.min),this.max.max(e.max),this}applyMatrix4(e){return this.isEmpty()?this:(Qn[0].set(this.min.x,this.min.y,this.min.z).applyMatrix4(e),Qn[1].set(this.min.x,this.min.y,this.max.z).applyMatrix4(e),Qn[2].set(this.min.x,this.max.y,this.min.z).applyMatrix4(e),Qn[3].set(this.min.x,this.max.y,this.max.z).applyMatrix4(e),Qn[4].set(this.max.x,this.min.y,this.min.z).applyMatrix4(e),Qn[5].set(this.max.x,this.min.y,this.max.z).applyMatrix4(e),Qn[6].set(this.max.x,this.max.y,this.min.z).applyMatrix4(e),Qn[7].set(this.max.x,this.max.y,this.max.z).applyMatrix4(e),this.setFromPoints(Qn),this)}translate(e){return this.min.add(e),this.max.add(e),this}equals(e){return e.min.equals(this.min)&&e.max.equals(this.max)}toJSON(){return{min:this.min.toArray(),max:this.max.toArray()}}fromJSON(e){return this.min.fromArray(e.min),this.max.fromArray(e.max),this}},Qn=[new L,new L,new L,new L,new L,new L,new L,new L],En=new L,bo=new Mt,ds=new L,ps=new L,ms=new L,vi=new L,Si=new L,Oi=new L,ir=new L,wo=new L,Ao=new L,Bi=new L;function Jl(s,e,t,n,i){for(let r=0,o=s.length-3;r<=o;r+=3){Bi.fromArray(s,r);let a=i.x*Math.abs(Bi.x)+i.y*Math.abs(Bi.y)+i.z*Math.abs(Bi.z),l=e.dot(Bi),c=t.dot(Bi),u=n.dot(Bi);if(Math.max(-Math.max(l,c,u),Math.min(l,c,u))>a)return!1}return!0}var It=new L,To=new pe,Cp=0,gt=class extends Bn{constructor(e,t,n=!1){if(super(),Array.isArray(e))throw new TypeError("THREE.BufferAttribute: array should be a Typed Array.");this.isBufferAttribute=!0,Object.defineProperty(this,"id",{value:Cp++}),this.name="",this.array=e,this.itemSize=t,this.count=e!==void 0?e.length/t:0,this.normalized=n,this.usage=Mf,this.updateRanges=[],this.gpuType=dn,this.version=0,this._normalizationScale=e!==void 0?Zc(e):void 0}onUploadCallback(){}set needsUpdate(e){e===!0&&this.version++}setUsage(e){return this.usage=e,this}addUpdateRange(e,t){this.updateRanges.push({start:e,count:t})}clearUpdateRanges(){this.updateRanges.length=0}copy(e){return this.name=e.name,this.array=new e.array.constructor(e.array),this.itemSize=e.itemSize,this.count=e.count,this.normalized=e.normalized,this.usage=e.usage,this.gpuType=e.gpuType,this._normalizationScale=e._normalizationScale,this}copyAt(e,t,n){e*=this.itemSize,n*=t.itemSize;for(let i=0,r=this.itemSize;i<r;i++)this.array[e+i]=t.array[n+i];return this}copyArray(e){return this.array.set(e),this}applyMatrix3(e){if(this.itemSize===2)for(let t=0,n=this.count;t<n;t++)To.fromBufferAttribute(this,t),To.applyMatrix3(e),this.setXY(t,To.x,To.y);else if(this.itemSize===3)for(let t=0,n=this.count;t<n;t++)It.fromBufferAttribute(this,t),It.applyMatrix3(e),this.setXYZ(t,It.x,It.y,It.z);return this}applyMatrix4(e){for(let t=0,n=this.count;t<n;t++)It.fromBufferAttribute(this,t),It.applyMatrix4(e),this.setXYZ(t,It.x,It.y,It.z);return this}applyNormalMatrix(e){for(let t=0,n=this.count;t<n;t++)It.fromBufferAttribute(this,t),It.applyNormalMatrix(e),this.setXYZ(t,It.x,It.y,It.z);return this}transformDirection(e){for(let t=0,n=this.count;t<n;t++)It.fromBufferAttribute(this,t),It.transformDirection(e),this.setXYZ(t,It.x,It.y,It.z);return this}set(e,t=0){return this.array.set(e,t),this}getComponent(e,t){let n=this.array[e*this.itemSize+t];return this.normalized&&(n=this._denormalize(n)),n}setComponent(e,t,n){return this.normalized&&(n=this._normalize(n)),this.array[e*this.itemSize+t]=n,this}getX(e){let t=this.array[e*this.itemSize];return this.normalized&&(t=this._denormalize(t)),t}setX(e,t){return this.normalized&&(t=this._normalize(t)),this.array[e*this.itemSize]=t,this}getY(e){let t=this.array[e*this.itemSize+1];return this.normalized&&(t=this._denormalize(t)),t}setY(e,t){return this.normalized&&(t=this._normalize(t)),this.array[e*this.itemSize+1]=t,this}getZ(e){let t=this.array[e*this.itemSize+2];return this.normalized&&(t=this._denormalize(t)),t}setZ(e,t){return this.normalized&&(t=this._normalize(t)),this.array[e*this.itemSize+2]=t,this}getW(e){let t=this.array[e*this.itemSize+3];return this.normalized&&(t=this._denormalize(t)),t}setW(e,t){return this.normalized&&(t=this._normalize(t)),this.array[e*this.itemSize+3]=t,this}setXY(e,t,n){return e*=this.itemSize,this.normalized&&(t=this._normalize(t),n=this._normalize(n)),this.array[e+0]=t,this.array[e+1]=n,this}setXYZ(e,t,n,i){return e*=this.itemSize,this.normalized&&(t=this._normalize(t),n=this._normalize(n),i=this._normalize(i)),this.array[e+0]=t,this.array[e+1]=n,this.array[e+2]=i,this}setXYZW(e,t,n,i,r){return e*=this.itemSize,this.normalized&&(t=this._normalize(t),n=this._normalize(n),i=this._normalize(i),r=this._normalize(r)),this.array[e+0]=t,this.array[e+1]=n,this.array[e+2]=i,this.array[e+3]=r,this}onUpload(e){return this.onUploadCallback=e,this}clone(){return new this.constructor(this.array,this.itemSize).copy(this)}toJSON(){let e={itemSize:this.itemSize,type:this.array.constructor.name,array:Array.from(this.array),normalized:this.normalized};return e.name=this.name,e.usage=this.usage,e.gpuType=this.gpuType,e}dispose(){this.dispatchEvent({type:"dispose"})}_denormalize(e){let t=this._normalizationScale;return t!==void 0?Math.max(e/t,-1):_p(e,this.array)}_normalize(e){let t=this._normalizationScale;return t!==void 0?Math.round(e*t):xp(e,this.array)}};var _r=class extends gt{constructor(e,t,n){super(new Uint16Array(e),t,n)}};var xr=class extends gt{constructor(e,t,n){super(new Uint32Array(e),t,n)}};var ct=class extends gt{constructor(e,t,n){super(new Float32Array(e),t,n)}},Pp=new Mt,sr=new L,$l=new L,zn=class{constructor(e=new L,t=-1){this.isSphere=!0,this.center=e,this.radius=t}set(e,t){return this.center.copy(e),this.radius=t,this}setFromPoints(e,t){let n=this.center;t!==void 0?n.copy(t):Pp.setFromPoints(e).getCenter(n);let i=0;for(let r=0,o=e.length;r<o;r++)i=Math.max(i,n.distanceToSquared(e[r]));return this.radius=Math.sqrt(i),this}copy(e){return this.center.copy(e.center),this.radius=e.radius,this}isEmpty(){return this.radius<0}makeEmpty(){return this.center.set(0,0,0),this.radius=-1,this}containsPoint(e){return e.distanceToSquared(this.center)<=this.radius*this.radius}distanceToPoint(e){return e.distanceTo(this.center)-this.radius}intersectsSphere(e){let t=this.radius+e.radius;return e.center.distanceToSquared(this.center)<=t*t}intersectsBox(e){return e.intersectsSphere(this)}intersectsPlane(e){return Math.abs(e.distanceToPoint(this.center))<=this.radius}clampPoint(e,t){let n=this.center.distanceToSquared(e);return t.copy(e),n>this.radius*this.radius&&(t.sub(this.center).normalize(),t.multiplyScalar(this.radius).add(this.center)),t}getBoundingBox(e){return this.isEmpty()?(e.makeEmpty(),e):(e.set(this.center,this.center),e.expandByScalar(this.radius),e)}applyMatrix4(e){return this.center.applyMatrix4(e),this.radius=this.radius*e.getMaxScaleOnAxis(),this}translate(e){return this.center.add(e),this}expandByPoint(e){if(this.isEmpty())return this.center.copy(e),this.radius=0,this;sr.subVectors(e,this.center);let t=sr.lengthSq();if(t>this.radius*this.radius){let n=Math.sqrt(t),i=(n-this.radius)*.5;this.center.addScaledVector(sr,i/n),this.radius+=i}return this}union(e){return e.isEmpty()?this:this.isEmpty()?(this.copy(e),this):(this.center.equals(e.center)===!0?this.radius=Math.max(this.radius,e.radius):($l.subVectors(e.center,this.center).setLength(e.radius),this.expandByPoint(sr.copy(e.center).add($l)),this.expandByPoint(sr.copy(e.center).sub($l))),this)}equals(e){return e.center.equals(this.center)&&e.radius===this.radius}clone(){return new this.constructor().copy(this)}toJSON(){return{radius:this.radius,center:this.center.toArray()}}fromJSON(e){return this.radius=e.radius,this.center.fromArray(e.center),this}},Ip=0,gn=new ze,Kl=new At,gs=new L,ln=new Mt,rr=new Mt,kt=new L,Lt=class s extends Bn{constructor(){super(),this.isBufferGeometry=!0,Object.defineProperty(this,"id",{value:Ip++}),this.uuid=Pi(),this.name="",this.type="BufferGeometry",this.index=null,this.indirect=null,this.indirectOffset=0,this.attributes={},this.morphAttributes={},this.morphTargetsRelative=!1,this.groups=[],this.boundingBox=null,this.boundingSphere=null,this.drawRange={start:0,count:1/0},this.userData={},this._transformed=!1}getIndex(){return this.index}setIndex(e){return Array.isArray(e)?this.index=new(pp(e)?xr:_r)(e,1):this.index=e,this}setIndirect(e,t=0){return this.indirect=e,this.indirectOffset=t,this}getIndirect(){return this.indirect}getAttribute(e){return this.attributes[e]}setAttribute(e,t){return this.attributes[e]=t,this}deleteAttribute(e){return delete this.attributes[e],this}hasAttribute(e){return this.attributes[e]!==void 0}addGroup(e,t,n=0){this.groups.push({start:e,count:t,materialIndex:n})}clearGroups(){this.groups=[]}setDrawRange(e,t){this.drawRange.start=e,this.drawRange.count=t}applyMatrix4(e){let t=this.attributes.position;t!==void 0&&(t.applyMatrix4(e),t.needsUpdate=!0);let n=this.attributes.normal;if(n!==void 0){let r=new Ze().getNormalMatrix(e);n.applyNormalMatrix(r),n.needsUpdate=!0}let i=this.attributes.tangent;return i!==void 0&&(i.transformDirection(e),i.needsUpdate=!0),this.boundingBox!==null&&this.computeBoundingBox(),this.boundingSphere!==null&&this.computeBoundingSphere(),this._transformed=!0,this}applyQuaternion(e){return gn.makeRotationFromQuaternion(e),this.applyMatrix4(gn),this}rotateX(e){return gn.makeRotationX(e),this.applyMatrix4(gn),this}rotateY(e){return gn.makeRotationY(e),this.applyMatrix4(gn),this}rotateZ(e){return gn.makeRotationZ(e),this.applyMatrix4(gn),this}translate(e,t,n){return gn.makeTranslation(e,t,n),this.applyMatrix4(gn),this}scale(e,t,n){return gn.makeScale(e,t,n),this.applyMatrix4(gn),this}lookAt(e){return Kl.lookAt(e),Kl.updateMatrix(),this.applyMatrix4(Kl.matrix),this}center(){return this.computeBoundingBox(),this.boundingBox.getCenter(gs).negate(),this.translate(gs.x,gs.y,gs.z),this}setFromPoints(e){let t=this.getAttribute("position");if(t===void 0){let n=[];for(let i=0,r=e.length;i<r;i++){let o=e[i];n.push(o.x,o.y,o.z||0)}this.setAttribute("position",new ct(n,3))}else{let n=Math.min(e.length,t.count);for(let i=0;i<n;i++){let r=e[i];t.setXYZ(i,r.x,r.y,r.z||0)}e.length>t.count&&Xe("BufferGeometry: Buffer size too small for points data. Use .dispose() and create a new geometry."),t.needsUpdate=!0}return this}computeBoundingBox(){this.boundingBox===null&&(this.boundingBox=new Mt);let e=this.attributes.position,t=this.morphAttributes.position;if(e&&e.isGLBufferAttribute){Ye("BufferGeometry.computeBoundingBox(): GLBufferAttribute requires a manual bounding box.",this),this.boundingBox.set(new L(-1/0,-1/0,-1/0),new L(1/0,1/0,1/0));return}if(e!==void 0){if(this.boundingBox.setFromBufferAttribute(e),t)for(let n=0,i=t.length;n<i;n++){let r=t[n];ln.setFromBufferAttribute(r),this.morphTargetsRelative?(kt.addVectors(this.boundingBox.min,ln.min),this.boundingBox.expandByPoint(kt),kt.addVectors(this.boundingBox.max,ln.max),this.boundingBox.expandByPoint(kt)):(this.boundingBox.expandByPoint(ln.min),this.boundingBox.expandByPoint(ln.max))}}else this.boundingBox.makeEmpty();(isNaN(this.boundingBox.min.x)||isNaN(this.boundingBox.min.y)||isNaN(this.boundingBox.min.z))&&Ye('BufferGeometry.computeBoundingBox(): Computed min/max have NaN values. The "position" attribute is likely to have NaN values.',this)}computeBoundingSphere(){this.boundingSphere===null&&(this.boundingSphere=new zn);let e=this.attributes.position,t=this.morphAttributes.position;if(e&&e.isGLBufferAttribute){Ye("BufferGeometry.computeBoundingSphere(): GLBufferAttribute requires a manual bounding sphere.",this),this.boundingSphere.set(new L,1/0);return}if(e){let n=this.boundingSphere.center;if(ln.setFromBufferAttribute(e),t)for(let r=0,o=t.length;r<o;r++){let a=t[r];rr.setFromBufferAttribute(a),this.morphTargetsRelative?(kt.addVectors(ln.min,rr.min),ln.expandByPoint(kt),kt.addVectors(ln.max,rr.max),ln.expandByPoint(kt)):(ln.expandByPoint(rr.min),ln.expandByPoint(rr.max))}ln.getCenter(n);let i=0;for(let r=0,o=e.count;r<o;r++)kt.fromBufferAttribute(e,r),i=Math.max(i,n.distanceToSquared(kt));if(t)for(let r=0,o=t.length;r<o;r++){let a=t[r],l=this.morphTargetsRelative;for(let c=0,u=a.count;c<u;c++)kt.fromBufferAttribute(a,c),l&&(gs.fromBufferAttribute(e,c),kt.add(gs)),i=Math.max(i,n.distanceToSquared(kt))}this.boundingSphere.radius=Math.sqrt(i),isNaN(this.boundingSphere.radius)&&Ye('BufferGeometry.computeBoundingSphere(): Computed radius is NaN. The "position" attribute is likely to have NaN values.',this)}}computeTangents(){let e=this.index,t=this.attributes;if(e===null||t.position===void 0||t.normal===void 0||t.uv===void 0){Ye("BufferGeometry: .computeTangents() failed. Missing required attributes (index, position, normal or uv)");return}let n=t.position,i=t.normal,r=t.uv,o=this.getAttribute("tangent");(o===void 0||o.count!==n.count)&&(o=new gt(new Float32Array(4*n.count),4),this.setAttribute("tangent",o));let a=[],l=[],c=new Uint8Array(n.count);for(let C=0;C<n.count;C++)a[C]=new L,l[C]=new L;let u=new L,h=new L,d=new L,f=new pe,m=new pe,_=new pe,p=new L,g=new L;function S(C,E,R){c[C]=c[E]=c[R]=1,u.fromBufferAttribute(n,C),h.fromBufferAttribute(n,E),d.fromBufferAttribute(n,R),f.fromBufferAttribute(r,C),m.fromBufferAttribute(r,E),_.fromBufferAttribute(r,R),h.sub(u),d.sub(u),m.sub(f),_.sub(f);let D=1/(m.x*_.y-_.x*m.y);isFinite(D)&&(p.copy(h).multiplyScalar(_.y).addScaledVector(d,-m.y).multiplyScalar(D),g.copy(d).multiplyScalar(m.x).addScaledVector(h,-_.x).multiplyScalar(D),a[C].add(p),a[E].add(p),a[R].add(p),l[C].add(g),l[E].add(g),l[R].add(g))}let v=this.groups;v.length===0&&(v=[{start:0,count:e.count}]);for(let C=0,E=v.length;C<E;++C){let R=v[C],D=R.start,k=R.count;for(let P=D,O=D+k;P<O;P+=3)S(e.getX(P+0),e.getX(P+1),e.getX(P+2))}let x=new L,b=new L,w=new L,y=new L;function M(C){w.fromBufferAttribute(i,C),y.copy(w);let E=a[C];x.copy(E),x.sub(w.multiplyScalar(w.dot(E))).normalize(),b.crossVectors(y,E);let D=b.dot(l[C])<0?-1:1;o.setXYZW(C,x.x,x.y,x.z,D)}for(let C=0,E=n.count;C<E;++C)c[C]&&M(C);this._transformed=!0}computeVertexNormals(){let e=this.index,t=this.getAttribute("position");if(t!==void 0){let n=this.getAttribute("normal");if(n===void 0||n.count!==t.count)n=new gt(new Float32Array(t.count*3),3),this.setAttribute("normal",n);else for(let d=0,f=n.count;d<f;d++)n.setXYZ(d,0,0,0);let i=new L,r=new L,o=new L,a=new L,l=new L,c=new L,u=new L,h=new L;if(e)for(let d=0,f=e.count;d<f;d+=3){let m=e.getX(d+0),_=e.getX(d+1),p=e.getX(d+2);i.fromBufferAttribute(t,m),r.fromBufferAttribute(t,_),o.fromBufferAttribute(t,p),u.subVectors(o,r),h.subVectors(i,r),u.cross(h),a.fromBufferAttribute(n,m),l.fromBufferAttribute(n,_),c.fromBufferAttribute(n,p),a.add(u),l.add(u),c.add(u),n.setXYZ(m,a.x,a.y,a.z),n.setXYZ(_,l.x,l.y,l.z),n.setXYZ(p,c.x,c.y,c.z)}else for(let d=0,f=t.count;d<f;d+=3)i.fromBufferAttribute(t,d+0),r.fromBufferAttribute(t,d+1),o.fromBufferAttribute(t,d+2),u.subVectors(o,r),h.subVectors(i,r),u.cross(h),n.setXYZ(d+0,u.x,u.y,u.z),n.setXYZ(d+1,u.x,u.y,u.z),n.setXYZ(d+2,u.x,u.y,u.z);this.normalizeNormals(),n.needsUpdate=!0}}normalizeNormals(){let e=this.attributes.normal;for(let t=0,n=e.count;t<n;t++)kt.fromBufferAttribute(e,t),kt.normalize(),e.setXYZ(t,kt.x,kt.y,kt.z)}toNonIndexed(){function e(a,l){let c=a.array,u=a.itemSize,h=a.normalized,d=new c.constructor(l.length*u),f=0,m=0;for(let _=0,p=l.length;_<p;_++){a.isInterleavedBufferAttribute?f=l[_]*a.data.stride+a.offset:f=l[_]*u;for(let g=0;g<u;g++)d[m++]=c[f++]}return new gt(d,u,h)}if(this.index===null)return Xe("BufferGeometry.toNonIndexed(): BufferGeometry is already non-indexed."),this;let t=new s,n=this.index.array,i=this.attributes;for(let a in i){let l=i[a],c=e(l,n);t.setAttribute(a,c)}let r=this.morphAttributes;for(let a in r){let l=[],c=r[a];for(let u=0,h=c.length;u<h;u++){let d=c[u],f=e(d,n);l.push(f)}t.morphAttributes[a]=l}t.morphTargetsRelative=this.morphTargetsRelative;let o=this.groups;for(let a=0,l=o.length;a<l;a++){let c=o[a];t.addGroup(c.start,c.count,c.materialIndex)}return t}toJSON(){let e={metadata:{version:4.7,type:"BufferGeometry",generator:"BufferGeometry.toJSON"}};if(e.uuid=this.uuid,e.type=this.parameters!==void 0&&this._transformed===!0?"BufferGeometry":this.type,e.name=this.name,Object.keys(this.userData).length>0&&(e.userData=this.userData),this.parameters!==void 0&&this._transformed!==!0){let l=this.parameters;for(let c in l)l[c]!==void 0&&(e[c]=l[c]);return e}e.data={attributes:{}};let t=this.index;t!==null&&(e.data.index={type:t.array.constructor.name,array:Array.prototype.slice.call(t.array)});let n=this.attributes;for(let l in n){let c=n[l];e.data.attributes[l]=c.toJSON(e.data)}let i={},r=!1;for(let l in this.morphAttributes){let c=this.morphAttributes[l],u=[];for(let h=0,d=c.length;h<d;h++){let f=c[h];u.push(f.toJSON(e.data))}u.length>0&&(i[l]=u,r=!0)}r&&(e.data.morphAttributes=i,e.data.morphTargetsRelative=this.morphTargetsRelative);let o=this.groups;o.length>0&&(e.data.groups=JSON.parse(JSON.stringify(o)));let a=this.boundingSphere;return a!==null&&(e.data.boundingSphere=a.toJSON()),e}clone(){return new this.constructor().copy(this)}copy(e){this.index=null,this.attributes={},this.morphAttributes={},this.groups=[],this.boundingBox=null,this.boundingSphere=null;let t={};this.name=e.name;let n=e.index;n!==null&&this.setIndex(n.clone());let i=e.attributes;for(let c in i){let u=i[c];this.setAttribute(c,u.clone(t))}let r=e.morphAttributes;for(let c in r){let u=[],h=r[c];for(let d=0,f=h.length;d<f;d++)u.push(h[d].clone(t));this.morphAttributes[c]=u}this.morphTargetsRelative=e.morphTargetsRelative;let o=e.groups;for(let c=0,u=o.length;c<u;c++){let h=o[c];this.addGroup(h.start,h.count,h.materialIndex)}let a=e.boundingBox;a!==null&&(this.boundingBox=a.clone());let l=e.boundingSphere;return l!==null&&(this.boundingSphere=l.clone()),this.drawRange.start=e.drawRange.start,this.drawRange.count=e.drawRange.count,this.userData=e.userData,this._transformed=e._transformed,this}dispose(){this.dispatchEvent({type:"dispose"})}};var jl=new L,Dp=new L,Lp=new Ze,tn=class{constructor(e=new L(1,0,0),t=0){this.isPlane=!0,this.normal=e,this.constant=t}set(e,t){return this.normal.copy(e),this.constant=t,this}setComponents(e,t,n,i){return this.normal.set(e,t,n),this.constant=i,this}setFromNormalAndCoplanarPoint(e,t){return this.normal.copy(e),this.constant=-t.dot(this.normal),this}setFromCoplanarPoints(e,t,n){let i=jl.subVectors(n,t).cross(Dp.subVectors(e,t)).normalize();return this.setFromNormalAndCoplanarPoint(i,e),this}copy(e){return this.normal.copy(e.normal),this.constant=e.constant,this}normalize(){let e=1/this.normal.length();return this.normal.multiplyScalar(e),this.constant*=e,this}negate(){return this.constant*=-1,this.normal.negate(),this}distanceToPoint(e){return this.normal.dot(e)+this.constant}distanceToSphere(e){return this.distanceToPoint(e.center)-e.radius}projectPoint(e,t){return t.copy(e).addScaledVector(this.normal,-this.distanceToPoint(e))}intersectLine(e,t,n=!0){let i=e.delta(jl),r=this.normal.dot(i);if(r===0)return this.distanceToPoint(e.start)===0?t.copy(e.start):null;let o=-(e.start.dot(this.normal)+this.constant)/r;return n===!0&&(o<0||o>1)?null:t.copy(e.start).addScaledVector(i,o)}intersectsLine(e){let t=this.distanceToPoint(e.start),n=this.distanceToPoint(e.end);return t<0&&n>0||n<0&&t>0}intersectsBox(e){return e.intersectsPlane(this)}intersectsSphere(e){return e.intersectsPlane(this)}coplanarPoint(e){return e.copy(this.normal).multiplyScalar(-this.constant)}applyMatrix4(e,t){let n=t||Lp.getNormalMatrix(e),i=this.coplanarPoint(jl).applyMatrix4(e),r=this.normal.applyMatrix3(n).normalize();return this.constant=-i.dot(r),this}translate(e){return this.constant-=e.dot(this.normal),this}equals(e){return e.normal.equals(this.normal)&&e.constant===this.constant}clone(){return new this.constructor().copy(this)}toJSON(){return{normal:this.normal.toArray(),constant:this.constant}}fromJSON(e){return this.normal.fromArray(e.normal),this.constant=e.constant,this}},Np=0,ri=class extends Bn{constructor(){super(),this.isMaterial=!0,Object.defineProperty(this,"id",{value:Np++}),this.uuid=Pi(),this.name="",this.type="Material",this.blending=Hs,this.side=Ri,this.vertexColors=!1,this.opacity=1,this.transparent=!1,this.alphaHash=!1,this.blendSrc=Ec,this.blendDst=Rc,this.blendEquation=Ki,this.blendSrcAlpha=null,this.blendDstAlpha=null,this.blendEquationAlpha=null,this.blendColor=new Ke(0,0,0),this.blendAlpha=0,this.depthFunc=bs,this.depthTest=!0,this.depthWrite=!0,this.stencilWriteMask=255,this.stencilFunc=gf,this.stencilRef=0,this.stencilFuncMask=255,this.stencilFail=Wo,this.stencilZFail=Wo,this.stencilZPass=Wo,this.stencilWrite=!1,this.clippingPlanes=null,this.clipIntersection=!1,this.clipShadows=!1,this.shadowSide=null,this.colorWrite=!0,this.precision=null,this.polygonOffset=!1,this.polygonOffsetFactor=0,this.polygonOffsetUnits=0,this.dithering=!1,this.alphaToCoverage=!1,this.premultipliedAlpha=!1,this.forceSinglePass=!1,this.allowOverride=!0,this.visible=!0,this.toneMapped=!0,this.userData={},this.version=0,this._alphaTest=0}get alphaTest(){return this._alphaTest}set alphaTest(e){this._alphaTest>0!=e>0&&this.version++,this._alphaTest=e}onBeforeRender(){}onBeforeCompile(){}customProgramCacheKey(){return this.onBeforeCompile.toString()}setValues(e){if(e!==void 0)for(let t in e){let n=e[t];if(n===void 0){Xe(`Material: parameter '${t}' has value of undefined.`);continue}let i=this[t];if(i===void 0){Xe(`Material: '${t}' is not a property of THREE.${this.type}.`);continue}i&&i.isColor?i.set(n):i&&i.isVector2&&n&&n.isVector2||i&&i.isEuler&&n&&n.isEuler||i&&i.isVector3&&n&&n.isVector3?i.copy(n):this[t]=n}}toJSON(e){let t=e===void 0||typeof e=="string";t&&(e={textures:{},images:{}});let n={metadata:{version:4.7,type:"Material",generator:"Material.toJSON"}};n.uuid=this.uuid,n.type=this.type,n.blending=this.blending,n.side=this.side,n.shadowSide=this.shadowSide,n.vertexColors=this.vertexColors,n.opacity=this.opacity,n.transparent=this.transparent,n.blendSrc=this.blendSrc,n.blendDst=this.blendDst,n.blendEquation=this.blendEquation,n.blendSrcAlpha=this.blendSrcAlpha,n.blendDstAlpha=this.blendDstAlpha,n.blendEquationAlpha=this.blendEquationAlpha,n.blendColor=this.blendColor.getHex(),n.blendAlpha=this.blendAlpha,n.depthFunc=this.depthFunc,n.depthTest=this.depthTest,n.depthWrite=this.depthWrite,n.colorWrite=this.colorWrite,n.clipIntersection=this.clipIntersection,n.clipShadows=this.clipShadows,n.stencilWriteMask=this.stencilWriteMask,n.stencilFunc=this.stencilFunc,n.stencilRef=this.stencilRef,n.stencilFuncMask=this.stencilFuncMask,n.stencilFail=this.stencilFail,n.stencilZFail=this.stencilZFail,n.stencilZPass=this.stencilZPass,n.stencilWrite=this.stencilWrite,n.polygonOffset=this.polygonOffset,n.polygonOffsetFactor=this.polygonOffsetFactor,n.polygonOffsetUnits=this.polygonOffsetUnits,n.dithering=this.dithering,n.alphaTest=this.alphaTest,n.alphaHash=this.alphaHash,n.alphaToCoverage=this.alphaToCoverage,n.premultipliedAlpha=this.premultipliedAlpha,n.forceSinglePass=this.forceSinglePass,n.allowOverride=this.allowOverride,n.visible=this.visible,n.toneMapped=this.toneMapped,n.name=this.name,this.color&&this.color.isColor&&(n.color=this.color.getHex()),this.roughness!==void 0&&(n.roughness=this.roughness),this.metalness!==void 0&&(n.metalness=this.metalness),this.sheen!==void 0&&(n.sheen=this.sheen),this.sheenColor&&this.sheenColor.isColor&&(n.sheenColor=this.sheenColor.getHex()),this.sheenRoughness!==void 0&&(n.sheenRoughness=this.sheenRoughness),this.emissive&&this.emissive.isColor&&(n.emissive=this.emissive.getHex()),this.emissiveIntensity!==void 0&&(n.emissiveIntensity=this.emissiveIntensity),this.specular&&this.specular.isColor&&(n.specular=this.specular.getHex()),this.specularIntensity!==void 0&&(n.specularIntensity=this.specularIntensity),this.specularColor&&this.specularColor.isColor&&(n.specularColor=this.specularColor.getHex()),this.shininess!==void 0&&(n.shininess=this.shininess),this.clearcoat!==void 0&&(n.clearcoat=this.clearcoat),this.clearcoatRoughness!==void 0&&(n.clearcoatRoughness=this.clearcoatRoughness),this.diffuseRoughness!==void 0&&(n.diffuseRoughness=this.diffuseRoughness),this.diffuseRoughnessMap&&this.diffuseRoughnessMap.isTexture&&(n.diffuseRoughnessMap=this.diffuseRoughnessMap.toJSON(e).uuid),this.clearcoatMap&&this.clearcoatMap.isTexture&&(n.clearcoatMap=this.clearcoatMap.toJSON(e).uuid),this.clearcoatRoughnessMap&&this.clearcoatRoughnessMap.isTexture&&(n.clearcoatRoughnessMap=this.clearcoatRoughnessMap.toJSON(e).uuid),this.clearcoatNormalMap&&this.clearcoatNormalMap.isTexture&&(n.clearcoatNormalMap=this.clearcoatNormalMap.toJSON(e).uuid,n.clearcoatNormalScale=this.clearcoatNormalScale.toArray()),this.sheenColorMap&&this.sheenColorMap.isTexture&&(n.sheenColorMap=this.sheenColorMap.toJSON(e).uuid),this.sheenRoughnessMap&&this.sheenRoughnessMap.isTexture&&(n.sheenRoughnessMap=this.sheenRoughnessMap.toJSON(e).uuid),this.dispersion!==void 0&&(n.dispersion=this.dispersion),this.retroreflectivity!==void 0&&(n.retroreflectivity=this.retroreflectivity),this.iridescence!==void 0&&(n.iridescence=this.iridescence),this.iridescenceIOR!==void 0&&(n.iridescenceIOR=this.iridescenceIOR),this.iridescenceThicknessMinimum!==void 0&&(n.iridescenceThicknessMinimum=this.iridescenceThicknessMinimum),this.iridescenceThicknessMaximum!==void 0&&(n.iridescenceThicknessMaximum=this.iridescenceThicknessMaximum),this.iridescenceMap&&this.iridescenceMap.isTexture&&(n.iridescenceMap=this.iridescenceMap.toJSON(e).uuid),this.iridescenceThicknessMap&&this.iridescenceThicknessMap.isTexture&&(n.iridescenceThicknessMap=this.iridescenceThicknessMap.toJSON(e).uuid),this.anisotropy!==void 0&&(n.anisotropy=this.anisotropy),this.anisotropyRotation!==void 0&&(n.anisotropyRotation=this.anisotropyRotation),this.anisotropyMap&&this.anisotropyMap.isTexture&&(n.anisotropyMap=this.anisotropyMap.toJSON(e).uuid),this.map&&this.map.isTexture&&(n.map=this.map.toJSON(e).uuid),this.matcap&&this.matcap.isTexture&&(n.matcap=this.matcap.toJSON(e).uuid),this.alphaMap&&this.alphaMap.isTexture&&(n.alphaMap=this.alphaMap.toJSON(e).uuid),this.lightMap&&this.lightMap.isTexture&&(n.lightMap=this.lightMap.toJSON(e).uuid,n.lightMapIntensity=this.lightMapIntensity),this.aoMap&&this.aoMap.isTexture&&(n.aoMap=this.aoMap.toJSON(e).uuid,n.aoMapIntensity=this.aoMapIntensity),this.bumpMap&&this.bumpMap.isTexture&&(n.bumpMap=this.bumpMap.toJSON(e).uuid,n.bumpScale=this.bumpScale),this.normalMap&&this.normalMap.isTexture&&(n.normalMap=this.normalMap.toJSON(e).uuid,n.normalMapType=this.normalMapType,n.normalScale=this.normalScale.toArray()),this.displacementMap&&this.displacementMap.isTexture&&(n.displacementMap=this.displacementMap.toJSON(e).uuid,n.displacementScale=this.displacementScale,n.displacementBias=this.displacementBias),this.roughnessMap&&this.roughnessMap.isTexture&&(n.roughnessMap=this.roughnessMap.toJSON(e).uuid),this.metalnessMap&&this.metalnessMap.isTexture&&(n.metalnessMap=this.metalnessMap.toJSON(e).uuid),this.emissiveMap&&this.emissiveMap.isTexture&&(n.emissiveMap=this.emissiveMap.toJSON(e).uuid),this.specularMap&&this.specularMap.isTexture&&(n.specularMap=this.specularMap.toJSON(e).uuid),this.specularIntensityMap&&this.specularIntensityMap.isTexture&&(n.specularIntensityMap=this.specularIntensityMap.toJSON(e).uuid),this.specularColorMap&&this.specularColorMap.isTexture&&(n.specularColorMap=this.specularColorMap.toJSON(e).uuid),this.envMap&&this.envMap.isTexture&&(n.envMap=this.envMap.toJSON(e).uuid,this.combine!==void 0&&(n.combine=this.combine)),this.envMapRotation!==void 0&&(n.envMapRotation=this.envMapRotation.toArray()),this.envMapIntensity!==void 0&&(n.envMapIntensity=this.envMapIntensity),this.reflectivity!==void 0&&(n.reflectivity=this.reflectivity),this.refractionRatio!==void 0&&(n.refractionRatio=this.refractionRatio),this.gradientMap&&this.gradientMap.isTexture&&(n.gradientMap=this.gradientMap.toJSON(e).uuid),this.transmission!==void 0&&(n.transmission=this.transmission),this.transmissionMap&&this.transmissionMap.isTexture&&(n.transmissionMap=this.transmissionMap.toJSON(e).uuid),this.thickness!==void 0&&(n.thickness=this.thickness),this.thicknessMap&&this.thicknessMap.isTexture&&(n.thicknessMap=this.thicknessMap.toJSON(e).uuid),this.attenuationDistance!==void 0&&(n.attenuationDistance=this.attenuationDistance),this.attenuationColor!==void 0&&(n.attenuationColor=this.attenuationColor.getHex()),this.size!==void 0&&(n.size=this.size),this.sizeAttenuation!==void 0&&(n.sizeAttenuation=this.sizeAttenuation),Array.isArray(this.clippingPlanes)&&this.clippingPlanes.length>0&&(n.clippingPlanes=this.clippingPlanes.map(r=>r.toJSON())),this.rotation!==void 0&&(n.rotation=this.rotation),this.depthPacking!==void 0&&(n.depthPacking=this.depthPacking),this.linewidth!==void 0&&(n.linewidth=this.linewidth),this.linecap!==void 0&&(n.linecap=this.linecap),this.linejoin!==void 0&&(n.linejoin=this.linejoin),this.dashSize!==void 0&&(n.dashSize=this.dashSize),this.gapSize!==void 0&&(n.gapSize=this.gapSize),this.scale!==void 0&&(n.scale=this.scale),this.wireframe!==void 0&&(n.wireframe=this.wireframe),this.wireframeLinewidth!==void 0&&(n.wireframeLinewidth=this.wireframeLinewidth),this.wireframeLinecap!==void 0&&(n.wireframeLinecap=this.wireframeLinecap),this.wireframeLinejoin!==void 0&&(n.wireframeLinejoin=this.wireframeLinejoin),this.flatShading!==void 0&&(n.flatShading=this.flatShading),this.fog!==void 0&&(n.fog=this.fog),Object.keys(this.userData).length>0&&(n.userData=this.userData);function i(r){let o=[];for(let a in r){let l=r[a];delete l.metadata,o.push(l)}return o}if(t){let r=i(e.textures),o=i(e.images);r.length>0&&(n.textures=r),o.length>0&&(n.images=o)}return n}fromJSON(e,t){if(e.uuid!==void 0&&(this.uuid=e.uuid),e.name!==void 0&&(this.name=e.name),e.color!==void 0&&this.color!==void 0&&this.color.setHex(e.color),e.roughness!==void 0&&(this.roughness=e.roughness),e.metalness!==void 0&&(this.metalness=e.metalness),e.sheen!==void 0&&(this.sheen=e.sheen),e.sheenColor!==void 0&&(this.sheenColor=new Ke().setHex(e.sheenColor)),e.sheenRoughness!==void 0&&(this.sheenRoughness=e.sheenRoughness),e.emissive!==void 0&&this.emissive!==void 0&&this.emissive.setHex(e.emissive),e.specular!==void 0&&this.specular!==void 0&&this.specular.setHex(e.specular),e.specularIntensity!==void 0&&(this.specularIntensity=e.specularIntensity),e.specularColor!==void 0&&this.specularColor!==void 0&&this.specularColor.setHex(e.specularColor),e.shininess!==void 0&&(this.shininess=e.shininess),e.clearcoat!==void 0&&(this.clearcoat=e.clearcoat),e.clearcoatRoughness!==void 0&&(this.clearcoatRoughness=e.clearcoatRoughness),e.diffuseRoughness!==void 0&&(this.diffuseRoughness=e.diffuseRoughness),e.dispersion!==void 0&&(this.dispersion=e.dispersion),e.retroreflectivity!==void 0&&(this.retroreflectivity=e.retroreflectivity),e.iridescence!==void 0&&(this.iridescence=e.iridescence),e.iridescenceIOR!==void 0&&(this.iridescenceIOR=e.iridescenceIOR),e.iridescenceThicknessMinimum!==void 0&&(this.iridescenceThicknessMinimum=e.iridescenceThicknessMinimum),e.iridescenceThicknessMaximum!==void 0&&(this.iridescenceThicknessMaximum=e.iridescenceThicknessMaximum),e.iridescenceThicknessRange!==void 0&&(this.iridescenceThicknessMinimum=e.iridescenceThicknessRange[0],this.iridescenceThicknessMaximum=e.iridescenceThicknessRange[1]),e.transmission!==void 0&&(this.transmission=e.transmission),e.thickness!==void 0&&(this.thickness=e.thickness),e.attenuationDistance!==void 0&&(this.attenuationDistance=e.attenuationDistance),e.attenuationColor!==void 0&&this.attenuationColor!==void 0&&this.attenuationColor.setHex(e.attenuationColor),e.anisotropy!==void 0&&(this.anisotropy=e.anisotropy),e.anisotropyRotation!==void 0&&(this.anisotropyRotation=e.anisotropyRotation),e.fog!==void 0&&(this.fog=e.fog),e.flatShading!==void 0&&(this.flatShading=e.flatShading),e.blending!==void 0&&(this.blending=e.blending),e.combine!==void 0&&(this.combine=e.combine),e.side!==void 0&&(this.side=e.side),e.shadowSide!==void 0&&(this.shadowSide=e.shadowSide),e.opacity!==void 0&&(this.opacity=e.opacity),e.transparent!==void 0&&(this.transparent=e.transparent),e.alphaTest!==void 0&&(this.alphaTest=e.alphaTest),e.alphaHash!==void 0&&(this.alphaHash=e.alphaHash),e.depthFunc!==void 0&&(this.depthFunc=e.depthFunc),e.depthTest!==void 0&&(this.depthTest=e.depthTest),e.depthWrite!==void 0&&(this.depthWrite=e.depthWrite),e.colorWrite!==void 0&&(this.colorWrite=e.colorWrite),e.clippingPlanes!==void 0&&(this.clippingPlanes=e.clippingPlanes.map(n=>new tn().fromJSON(n))),e.clipIntersection!==void 0&&(this.clipIntersection=e.clipIntersection),e.clipShadows!==void 0&&(this.clipShadows=e.clipShadows),e.depthPacking!==void 0&&(this.depthPacking=e.depthPacking),e.blendSrc!==void 0&&(this.blendSrc=e.blendSrc),e.blendDst!==void 0&&(this.blendDst=e.blendDst),e.blendEquation!==void 0&&(this.blendEquation=e.blendEquation),e.blendSrcAlpha!==void 0&&(this.blendSrcAlpha=e.blendSrcAlpha),e.blendDstAlpha!==void 0&&(this.blendDstAlpha=e.blendDstAlpha),e.blendEquationAlpha!==void 0&&(this.blendEquationAlpha=e.blendEquationAlpha),e.blendColor!==void 0&&this.blendColor!==void 0&&this.blendColor.setHex(e.blendColor),e.blendAlpha!==void 0&&(this.blendAlpha=e.blendAlpha),e.stencilWriteMask!==void 0&&(this.stencilWriteMask=e.stencilWriteMask),e.stencilFunc!==void 0&&(this.stencilFunc=e.stencilFunc),e.stencilRef!==void 0&&(this.stencilRef=e.stencilRef),e.stencilFuncMask!==void 0&&(this.stencilFuncMask=e.stencilFuncMask),e.stencilFail!==void 0&&(this.stencilFail=e.stencilFail),e.stencilZFail!==void 0&&(this.stencilZFail=e.stencilZFail),e.stencilZPass!==void 0&&(this.stencilZPass=e.stencilZPass),e.stencilWrite!==void 0&&(this.stencilWrite=e.stencilWrite),e.wireframe!==void 0&&(this.wireframe=e.wireframe),e.wireframeLinewidth!==void 0&&(this.wireframeLinewidth=e.wireframeLinewidth),e.wireframeLinecap!==void 0&&(this.wireframeLinecap=e.wireframeLinecap),e.wireframeLinejoin!==void 0&&(this.wireframeLinejoin=e.wireframeLinejoin),e.rotation!==void 0&&(this.rotation=e.rotation),e.linewidth!==void 0&&(this.linewidth=e.linewidth),e.linecap!==void 0&&(this.linecap=e.linecap),e.linejoin!==void 0&&(this.linejoin=e.linejoin),e.dashSize!==void 0&&(this.dashSize=e.dashSize),e.gapSize!==void 0&&(this.gapSize=e.gapSize),e.scale!==void 0&&(this.scale=e.scale),e.polygonOffset!==void 0&&(this.polygonOffset=e.polygonOffset),e.polygonOffsetFactor!==void 0&&(this.polygonOffsetFactor=e.polygonOffsetFactor),e.polygonOffsetUnits!==void 0&&(this.polygonOffsetUnits=e.polygonOffsetUnits),e.dithering!==void 0&&(this.dithering=e.dithering),e.alphaToCoverage!==void 0&&(this.alphaToCoverage=e.alphaToCoverage),e.premultipliedAlpha!==void 0&&(this.premultipliedAlpha=e.premultipliedAlpha),e.forceSinglePass!==void 0&&(this.forceSinglePass=e.forceSinglePass),e.allowOverride!==void 0&&(this.allowOverride=e.allowOverride),e.visible!==void 0&&(this.visible=e.visible),e.toneMapped!==void 0&&(this.toneMapped=e.toneMapped),e.userData!==void 0&&(this.userData=e.userData),e.vertexColors!==void 0&&(typeof e.vertexColors=="number"?this.vertexColors=e.vertexColors>0:this.vertexColors=e.vertexColors),e.size!==void 0&&(this.size=e.size),e.sizeAttenuation!==void 0&&(this.sizeAttenuation=e.sizeAttenuation),e.map!==void 0&&(this.map=t[e.map]||null),e.matcap!==void 0&&(this.matcap=t[e.matcap]||null),e.alphaMap!==void 0&&(this.alphaMap=t[e.alphaMap]||null),e.bumpMap!==void 0&&(this.bumpMap=t[e.bumpMap]||null),e.bumpScale!==void 0&&(this.bumpScale=e.bumpScale),e.normalMap!==void 0&&(this.normalMap=t[e.normalMap]||null),e.normalMapType!==void 0&&(this.normalMapType=e.normalMapType),e.normalScale!==void 0){let n=e.normalScale;Array.isArray(n)===!1&&(n=[n,n]),this.normalScale=new pe().fromArray(n)}return e.displacementMap!==void 0&&(this.displacementMap=t[e.displacementMap]||null),e.displacementScale!==void 0&&(this.displacementScale=e.displacementScale),e.displacementBias!==void 0&&(this.displacementBias=e.displacementBias),e.roughnessMap!==void 0&&(this.roughnessMap=t[e.roughnessMap]||null),e.metalnessMap!==void 0&&(this.metalnessMap=t[e.metalnessMap]||null),e.emissiveMap!==void 0&&(this.emissiveMap=t[e.emissiveMap]||null),e.emissiveIntensity!==void 0&&(this.emissiveIntensity=e.emissiveIntensity),e.specularMap!==void 0&&(this.specularMap=t[e.specularMap]||null),e.specularIntensityMap!==void 0&&(this.specularIntensityMap=t[e.specularIntensityMap]||null),e.specularColorMap!==void 0&&(this.specularColorMap=t[e.specularColorMap]||null),e.envMap!==void 0&&(this.envMap=t[e.envMap]||null),e.envMapRotation!==void 0&&this.envMapRotation.fromArray(e.envMapRotation),e.envMapIntensity!==void 0&&(this.envMapIntensity=e.envMapIntensity),e.reflectivity!==void 0&&(this.reflectivity=e.reflectivity),e.refractionRatio!==void 0&&(this.refractionRatio=e.refractionRatio),e.lightMap!==void 0&&(this.lightMap=t[e.lightMap]||null),e.lightMapIntensity!==void 0&&(this.lightMapIntensity=e.lightMapIntensity),e.aoMap!==void 0&&(this.aoMap=t[e.aoMap]||null),e.aoMapIntensity!==void 0&&(this.aoMapIntensity=e.aoMapIntensity),e.gradientMap!==void 0&&(this.gradientMap=t[e.gradientMap]||null),e.clearcoatMap!==void 0&&(this.clearcoatMap=t[e.clearcoatMap]||null),e.clearcoatRoughnessMap!==void 0&&(this.clearcoatRoughnessMap=t[e.clearcoatRoughnessMap]||null),e.clearcoatNormalMap!==void 0&&(this.clearcoatNormalMap=t[e.clearcoatNormalMap]||null),e.clearcoatNormalScale!==void 0&&(this.clearcoatNormalScale=new pe().fromArray(e.clearcoatNormalScale)),e.diffuseRoughnessMap!==void 0&&(this.diffuseRoughnessMap=t[e.diffuseRoughnessMap]||null),e.iridescenceMap!==void 0&&(this.iridescenceMap=t[e.iridescenceMap]||null),e.iridescenceThicknessMap!==void 0&&(this.iridescenceThicknessMap=t[e.iridescenceThicknessMap]||null),e.transmissionMap!==void 0&&(this.transmissionMap=t[e.transmissionMap]||null),e.thicknessMap!==void 0&&(this.thicknessMap=t[e.thicknessMap]||null),e.anisotropyMap!==void 0&&(this.anisotropyMap=t[e.anisotropyMap]||null),e.sheenColorMap!==void 0&&(this.sheenColorMap=t[e.sheenColorMap]||null),e.sheenRoughnessMap!==void 0&&(this.sheenRoughnessMap=t[e.sheenRoughnessMap]||null),this}clone(){return new this.constructor().copy(this)}copy(e){this.name=e.name,this.blending=e.blending,this.side=e.side,this.vertexColors=e.vertexColors,this.opacity=e.opacity,this.transparent=e.transparent,this.blendSrc=e.blendSrc,this.blendDst=e.blendDst,this.blendEquation=e.blendEquation,this.blendSrcAlpha=e.blendSrcAlpha,this.blendDstAlpha=e.blendDstAlpha,this.blendEquationAlpha=e.blendEquationAlpha,this.blendColor.copy(e.blendColor),this.blendAlpha=e.blendAlpha,this.depthFunc=e.depthFunc,this.depthTest=e.depthTest,this.depthWrite=e.depthWrite,this.stencilWriteMask=e.stencilWriteMask,this.stencilFunc=e.stencilFunc,this.stencilRef=e.stencilRef,this.stencilFuncMask=e.stencilFuncMask,this.stencilFail=e.stencilFail,this.stencilZFail=e.stencilZFail,this.stencilZPass=e.stencilZPass,this.stencilWrite=e.stencilWrite;let t=e.clippingPlanes,n=null;if(t!==null){let i=t.length;n=new Array(i);for(let r=0;r!==i;++r)n[r]=t[r].clone()}return this.clippingPlanes=n,this.clipIntersection=e.clipIntersection,this.clipShadows=e.clipShadows,this.shadowSide=e.shadowSide,this.colorWrite=e.colorWrite,this.precision=e.precision,this.polygonOffset=e.polygonOffset,this.polygonOffsetFactor=e.polygonOffsetFactor,this.polygonOffsetUnits=e.polygonOffsetUnits,this.dithering=e.dithering,this.alphaTest=e.alphaTest,this.alphaHash=e.alphaHash,this.alphaToCoverage=e.alphaToCoverage,this.premultipliedAlpha=e.premultipliedAlpha,this.forceSinglePass=e.forceSinglePass,this.allowOverride=e.allowOverride,this.visible=e.visible,this.toneMapped=e.toneMapped,this.userData=JSON.parse(JSON.stringify(e.userData)),this}dispose(){this.dispatchEvent({type:"dispose"})}set needsUpdate(e){e===!0&&this.version++}};var ei=new L,Ql=new L,Eo=new L,Ro=new L,Is=class{constructor(e=new L,t=new L(0,0,-1)){this.origin=e,this.direction=t}set(e,t){return this.origin.copy(e),this.direction.copy(t),this}copy(e){return this.origin.copy(e.origin),this.direction.copy(e.direction),this}at(e,t){return t.copy(this.origin).addScaledVector(this.direction,e)}lookAt(e){return this.direction.copy(e).sub(this.origin).normalize(),this}recast(e){return this.origin.copy(this.at(e,ei)),this}closestPointToPoint(e,t){t.subVectors(e,this.origin);let n=t.dot(this.direction);return n<0?t.copy(this.origin):t.copy(this.origin).addScaledVector(this.direction,n)}distanceToPoint(e){return Math.sqrt(this.distanceSqToPoint(e))}distanceSqToPoint(e){let t=ei.subVectors(e,this.origin).dot(this.direction);return t<0?this.origin.distanceToSquared(e):(ei.copy(this.origin).addScaledVector(this.direction,t),ei.distanceToSquared(e))}distanceSqToSegment(e,t,n,i){Ql.copy(e).add(t).multiplyScalar(.5),Eo.copy(t).sub(e).normalize(),Ro.copy(this.origin).sub(Ql);let r=e.distanceTo(t)*.5,o=-this.direction.dot(Eo),a=Ro.dot(this.direction),l=-Ro.dot(Eo),c=Ro.lengthSq(),u=Math.abs(1-o*o),h,d,f,m;if(u>0)if(h=o*l-a,d=o*a-l,m=r*u,h>=0)if(d>=-m)if(d<=m){let _=1/u;h*=_,d*=_,f=h*(h+o*d+2*a)+d*(o*h+d+2*l)+c}else d=r,h=Math.max(0,-(o*d+a)),f=-h*h+d*(d+2*l)+c;else d=-r,h=Math.max(0,-(o*d+a)),f=-h*h+d*(d+2*l)+c;else d<=-m?(h=Math.max(0,-(-o*r+a)),d=h>0?-r:Math.min(Math.max(-r,-l),r),f=-h*h+d*(d+2*l)+c):d<=m?(h=0,d=Math.min(Math.max(-r,-l),r),f=d*(d+2*l)+c):(h=Math.max(0,-(o*r+a)),d=h>0?r:Math.min(Math.max(-r,-l),r),f=-h*h+d*(d+2*l)+c);else d=o>0?-r:r,h=Math.max(0,-(o*d+a)),f=-h*h+d*(d+2*l)+c;return n&&n.copy(this.origin).addScaledVector(this.direction,h),i&&i.copy(Ql).addScaledVector(Eo,d),f}intersectSphere(e,t){if(e.radius<0)return null;ei.subVectors(e.center,this.origin);let n=ei.dot(this.direction),i=ei.dot(ei)-n*n,r=e.radius*e.radius;if(i>r)return null;let o=Math.sqrt(r-i),a=n-o,l=n+o;return l<0?null:a<0?this.at(l,t):this.at(a,t)}intersectsSphere(e){return e.radius<0?!1:this.distanceSqToPoint(e.center)<=e.radius*e.radius}distanceToPlane(e){let t=e.normal.dot(this.direction);if(t===0)return e.distanceToPoint(this.origin)===0?0:null;let n=-(this.origin.dot(e.normal)+e.constant)/t;return n>=0?n:null}intersectPlane(e,t){let n=this.distanceToPlane(e);return n===null?null:this.at(n,t)}intersectsPlane(e){let t=e.distanceToPoint(this.origin);return t===0||e.normal.dot(this.direction)*t<0}intersectBox(e,t){let n,i,r,o,a,l,c=1/this.direction.x,u=1/this.direction.y,h=1/this.direction.z,d=this.origin;return c>=0?(n=(e.min.x-d.x)*c,i=(e.max.x-d.x)*c):(n=(e.max.x-d.x)*c,i=(e.min.x-d.x)*c),u>=0?(r=(e.min.y-d.y)*u,o=(e.max.y-d.y)*u):(r=(e.max.y-d.y)*u,o=(e.min.y-d.y)*u),n>o||r>i||((r>n||isNaN(n))&&(n=r),(o<i||isNaN(i))&&(i=o),h>=0?(a=(e.min.z-d.z)*h,l=(e.max.z-d.z)*h):(a=(e.max.z-d.z)*h,l=(e.min.z-d.z)*h),n>l||a>i)||((a>n||n!==n)&&(n=a),(l<i||i!==i)&&(i=l),i<0)?null:this.at(n>=0?n:i,t)}intersectsBox(e){return this.intersectBox(e,ei)!==null}intersectTriangle(e,t,n,i,r){let o=this.origin,a=this.direction,l=a.x,c=a.y,u=a.z,h=e.x-o.x,d=e.y-o.y,f=e.z-o.z,m=t.x-o.x,_=t.y-o.y,p=t.z-o.z,g=n.x-o.x,S=n.y-o.y,v=n.z-o.z,x=Math.abs(l),b=Math.abs(c),w=Math.abs(u),y,M,C,E,R,D,k,P,O,G,U,ee;if(x>=b&&x>=w?(C=l,D=h,O=m,ee=g,l>=0?(y=c,M=u,E=d,R=f,k=_,P=p,G=S,U=v):(y=u,M=c,E=f,R=d,k=p,P=_,G=v,U=S)):b>=w?(C=c,D=d,O=_,ee=S,c>=0?(y=u,M=l,E=f,R=h,k=p,P=m,G=v,U=g):(y=l,M=u,E=h,R=f,k=m,P=p,G=g,U=v)):(C=u,D=f,O=p,ee=v,u>=0?(y=l,M=c,E=h,R=d,k=m,P=_,G=g,U=S):(y=c,M=l,E=d,R=h,k=_,P=m,G=S,U=g)),C===0)return null;let W=y/C,K=M/C,$=1/C,he=E-W*D,de=R-K*D,q=k-W*O,ie=P-K*O,ce=G-W*ee,Ce=U-K*ee,ue=ce*ie-Ce*q,H=he*Ce-de*ce,se=q*de-ie*he;if(i){if(ue<0||H<0||se<0)return null}else if((ue<0||H<0||se<0)&&(ue>0||H>0||se>0))return null;let fe=ue+H+se;if(fe===0)return null;let Le=$*(ue*D+H*O+se*ee);return(fe>0?Le<0:Le>0)?null:this.at(Le/fe,r)}applyMatrix4(e){return this.origin.applyMatrix4(e),this.direction.transformDirection(e),this}equals(e){return e.origin.equals(this.origin)&&e.direction.equals(this.direction)}clone(){return new this.constructor().copy(this)}},Wt=class extends ri{constructor(e){super(),this.isMeshBasicMaterial=!0,this.type="MeshBasicMaterial",this.color=new Ke(16777215),this.map=null,this.lightMap=null,this.lightMapIntensity=1,this.aoMap=null,this.aoMapIntensity=1,this.specularMap=null,this.alphaMap=null,this.envMap=null,this.envMapRotation=new Ut,this.combine=va,this.reflectivity=1,this.refractionRatio=.98,this.wireframe=!1,this.wireframeLinewidth=1,this.wireframeLinecap="round",this.wireframeLinejoin="round",this.fog=!0,this.setValues(e)}copy(e){return super.copy(e),this.color.copy(e.color),this.map=e.map,this.lightMap=e.lightMap,this.lightMapIntensity=e.lightMapIntensity,this.aoMap=e.aoMap,this.aoMapIntensity=e.aoMapIntensity,this.specularMap=e.specularMap,this.alphaMap=e.alphaMap,this.envMap=e.envMap,this.envMapRotation.copy(e.envMapRotation),this.combine=e.combine,this.reflectivity=e.reflectivity,this.refractionRatio=e.refractionRatio,this.wireframe=e.wireframe,this.wireframeLinewidth=e.wireframeLinewidth,this.wireframeLinecap=e.wireframeLinecap,this.wireframeLinejoin=e.wireframeLinejoin,this.fog=e.fog,this}},du=new ze,zi=new Is,Co=new zn,pu=new L,Po=new L,Io=new L,Do=new L,ec=new L,Lo=new L,mu=new L,No=new L,ke=class extends At{constructor(e=new Lt,t=new Wt){super(),this.isMesh=!0,this.type="Mesh",this.geometry=e,this.material=t,this.morphTargetDictionary=void 0,this.morphTargetInfluences=void 0,this.count=1,this.updateMorphTargets()}copy(e,t){return super.copy(e,t),e.morphTargetInfluences!==void 0?this.morphTargetInfluences=e.morphTargetInfluences.slice():this.morphTargetInfluences=void 0,e.morphTargetDictionary!==void 0?this.morphTargetDictionary=Object.assign({},e.morphTargetDictionary):this.morphTargetDictionary=void 0,this.material=Array.isArray(e.material)?e.material.slice():e.material,this.geometry=e.geometry,this}updateMorphTargets(){let t=this.geometry.morphAttributes,n=Object.keys(t);if(n.length>0){let i=t[n[0]];if(i!==void 0){this.morphTargetInfluences=[],this.morphTargetDictionary={};for(let r=0,o=i.length;r<o;r++){let a=i[r].name||String(r);this.morphTargetInfluences.push(0),this.morphTargetDictionary[a]=r}}}}getVertexPosition(e,t){let n=this.geometry,i=n.attributes.position,r=n.morphAttributes.position,o=n.morphTargetsRelative;t.fromBufferAttribute(i,e);let a=this.morphTargetInfluences;if(r&&a){Lo.set(0,0,0);for(let l=0,c=r.length;l<c;l++){let u=a[l],h=r[l];u!==0&&(ec.fromBufferAttribute(h,e),o?Lo.addScaledVector(ec,u):Lo.addScaledVector(ec.sub(t),u))}t.add(Lo)}return t}intersectsFrustum(e){return e.intersectsObject(this)}raycast(e,t){let n=this.geometry,i=this.material,r=this.matrixWorld;i!==void 0&&(n.boundingSphere===null&&n.computeBoundingSphere(),Co.copy(n.boundingSphere),Co.applyMatrix4(r),zi.copy(e.ray).recast(e.near),!(Co.containsPoint(zi.origin)===!1&&(zi.intersectSphere(Co,pu)===null||zi.origin.distanceToSquared(pu)>(e.far-e.near)**2))&&(du.copy(r).invert(),zi.copy(e.ray).applyMatrix4(du),!(n.boundingBox!==null&&zi.intersectsBox(n.boundingBox)===!1)&&this._computeIntersections(e,t,zi)))}_computeIntersections(e,t,n){let i,r=this.geometry,o=this.material,a=r.index,l=r.attributes.position,c=r.attributes.uv,u=r.attributes.uv1,h=r.attributes.normal,d=r.groups,f=r.drawRange;if(a!==null)if(Array.isArray(o))for(let m=0,_=d.length;m<_;m++){let p=d[m],g=o[p.materialIndex],S=Math.max(p.start,f.start),v=Math.min(a.count,Math.min(p.start+p.count,f.start+f.count));for(let x=S,b=v;x<b;x+=3){let w=a.getX(x),y=a.getX(x+1),M=a.getX(x+2);i=Uo(this,g,e,n,c,u,h,w,y,M),i&&(i.faceIndex=Math.floor(x/3),i.face.materialIndex=p.materialIndex,t.push(i))}}else{let m=Math.max(0,f.start),_=Math.min(a.count,f.start+f.count);for(let p=m,g=_;p<g;p+=3){let S=a.getX(p),v=a.getX(p+1),x=a.getX(p+2);i=Uo(this,o,e,n,c,u,h,S,v,x),i&&(i.faceIndex=Math.floor(p/3),t.push(i))}}else if(l!==void 0)if(Array.isArray(o))for(let m=0,_=d.length;m<_;m++){let p=d[m],g=o[p.materialIndex],S=Math.max(p.start,f.start),v=Math.min(l.count,Math.min(p.start+p.count,f.start+f.count));for(let x=S,b=v;x<b;x+=3){let w=x,y=x+1,M=x+2;i=Uo(this,g,e,n,c,u,h,w,y,M),i&&(i.faceIndex=Math.floor(x/3),i.face.materialIndex=p.materialIndex,t.push(i))}}else{let m=Math.max(0,f.start),_=Math.min(l.count,f.start+f.count);for(let p=m,g=_;p<g;p+=3){let S=p,v=p+1,x=p+2;i=Uo(this,o,e,n,c,u,h,S,v,x),i&&(i.faceIndex=Math.floor(p/3),t.push(i))}}}};function Up(s,e,t,n,i,r,o,a){let l;if(e.side===Xt?l=n.intersectTriangle(o,r,i,!0,a):l=n.intersectTriangle(i,r,o,e.side===Ri,a),l===null)return null;No.copy(a),No.applyMatrix4(s.matrixWorld);let c=t.ray.origin.distanceTo(No);return c<t.near||c>t.far?null:{distance:c,point:No.clone(),object:s}}function Uo(s,e,t,n,i,r,o,a,l,c){s.getVertexPosition(a,Po),s.getVertexPosition(l,Io),s.getVertexPosition(c,Do);let u=Up(s,e,t,n,Po,Io,Do,mu);if(u){let h=new L;On.getBarycoord(mu,Po,Io,Do,h),i&&(u.uv=On.getInterpolatedAttribute(i,a,l,c,h,new pe)),r&&(u.uv1=On.getInterpolatedAttribute(r,a,l,c,h,new pe)),o&&(u.normal=On.getInterpolatedAttribute(o,a,l,c,h,new L),u.normal.dot(n.direction)>0&&u.normal.multiplyScalar(-1));let d={a,b:l,c,normal:new L,materialIndex:0};On.getNormal(Po,Io,Do,d.normal),u.face=d,u.barycoord=h}return u}var or=new _t,gu=new _t,_u=new _t,Fp=new _t,xu=new ze,Fo=new L,tc=new zn,yu=new ze,nc=new Is,yr=class extends ke{constructor(e,t){super(e,t),this.isSkinnedMesh=!0,this.type="SkinnedMesh",this.bindMode=ac,this.bindMatrix=new ze,this.bindMatrixInverse=new ze,this.boundingBox=null,this.boundingSphere=null}computeBoundingBox(){let e=this.geometry;this.boundingBox===null&&(this.boundingBox=new Mt),this.boundingBox.makeEmpty();let t=e.getAttribute("position");for(let n=0;n<t.count;n++)this.getVertexPosition(n,Fo),this.boundingBox.expandByPoint(Fo)}computeBoundingSphere(){let e=this.geometry;this.boundingSphere===null&&(this.boundingSphere=new zn),this.boundingSphere.makeEmpty();let t=e.getAttribute("position");for(let n=0;n<t.count;n++)this.getVertexPosition(n,Fo),this.boundingSphere.expandByPoint(Fo)}copy(e,t){return super.copy(e,t),this.bindMode=e.bindMode,this.bindMatrix.copy(e.bindMatrix),this.bindMatrixInverse.copy(e.bindMatrixInverse),this.skeleton=e.skeleton,this.boundingBox=e.boundingBox!==null?e.boundingBox.clone():null,this.boundingSphere=e.boundingSphere!==null?e.boundingSphere.clone():null,this}raycast(e,t){let n=this.material,i=this.matrixWorld;n!==void 0&&(this.boundingSphere===null&&this.computeBoundingSphere(),tc.copy(this.boundingSphere),tc.applyMatrix4(i),e.ray.intersectsSphere(tc)!==!1&&(yu.copy(i).invert(),nc.copy(e.ray).applyMatrix4(yu),!(this.boundingBox!==null&&nc.intersectsBox(this.boundingBox)===!1)&&this._computeIntersections(e,t,nc)))}getVertexPosition(e,t){return super.getVertexPosition(e,t),this.applyBoneTransform(e,t),t}bind(e,t){this.skeleton=e,t===void 0&&(this.updateMatrixWorld(!0),this.skeleton.calculateInverses(),t=this.matrixWorld),this.bindMatrix.copy(t),this.bindMatrixInverse.copy(t).invert()}pose(){this.skeleton.pose()}normalizeSkinWeights(){let e=new _t,t=this.geometry.attributes.skinWeight;for(let n=0,i=t.count;n<i;n++){e.fromBufferAttribute(t,n);let r=1/e.manhattanLength();r!==1/0?e.multiplyScalar(r):e.set(1,0,0,0),t.setXYZW(n,e.x,e.y,e.z,e.w)}}updateMatrixWorld(e){super.updateMatrixWorld(e),this.bindMode===ac?this.bindMatrixInverse.copy(this.matrixWorld).invert():this.bindMode===uf?this.bindMatrixInverse.copy(this.bindMatrix).invert():Xe("SkinnedMesh: Unrecognized bindMode: "+this.bindMode)}applyBoneTransform(e,t){let n=this.skeleton,i=this.geometry;gu.fromBufferAttribute(i.attributes.skinIndex,e),_u.fromBufferAttribute(i.attributes.skinWeight,e),t.isVector4?(or.copy(t),t.set(0,0,0,0)):(or.set(t.x,t.y,t.z,1),t.set(0,0,0)),or.applyMatrix4(this.bindMatrix);for(let r=0;r<4;r++){let o=_u.getComponent(r);if(o!==0){let a=gu.getComponent(r);xu.multiplyMatrices(n.bones[a].matrixWorld,n.boneInverses[a]),t.addScaledVector(Fp.copy(or).applyMatrix4(xu),o)}}return t.isVector4&&(t.w=or.w),t.applyMatrix4(this.bindMatrixInverse)}},Ds=class extends At{constructor(){super(),this.isBone=!0,this.type="Bone"}},Ls=class extends Nt{constructor(e=null,t=1,n=1,i,r,o,a,l,c=Vt,u=Vt,h,d){super(null,o,a,l,c,u,i,r,h,d),this.isDataTexture=!0,this.image={data:e,width:t,height:n},this.generateMipmaps=!1,this.flipY=!1,this.unpackAlignment=1}},vu=new ze,Op=new ze,vr=class s{constructor(e=[],t=[]){this.uuid=Pi(),this.bones=e.slice(0),this.boneInverses=t,this.boneMatrices=null,this.previousBoneMatrices=null,this.boneTexture=null,this.previousBoneTexture=null,this.init()}init(){let e=this.bones,t=this.boneInverses;if(this.boneMatrices=new Float32Array(e.length*16),t.length===0)this.calculateInverses();else if(e.length!==t.length){Xe("Skeleton: Number of inverse bone matrices does not match amount of bones."),this.boneInverses=[];for(let n=0,i=this.bones.length;n<i;n++)this.boneInverses.push(new ze)}}calculateInverses(){this.boneInverses.length=0;for(let e=0,t=this.bones.length;e<t;e++){let n=new ze;this.bones[e]&&n.copy(this.bones[e].matrixWorld).invert(),this.boneInverses.push(n)}}pose(){for(let e=0,t=this.bones.length;e<t;e++){let n=this.bones[e];n&&n.matrixWorld.copy(this.boneInverses[e]).invert()}for(let e=0,t=this.bones.length;e<t;e++){let n=this.bones[e];n&&(n.parent&&n.parent.isBone?(n.matrix.copy(n.parent.matrixWorld).invert(),n.matrix.multiply(n.matrixWorld)):n.matrix.copy(n.matrixWorld),n.matrix.decompose(n.position,n.quaternion,n.scale))}}update(){let e=this.bones,t=this.boneInverses,n=this.boneMatrices,i=this.boneTexture;for(let r=0,o=e.length;r<o;r++){let a=e[r]?e[r].matrixWorld:Op;vu.multiplyMatrices(a,t[r]),vu.toArray(n,r*16)}i!==null&&(i.needsUpdate=!0)}clone(){return new s(this.bones,this.boneInverses)}computeBoneTexture(){let e=Math.sqrt(this.bones.length*4);e=Math.ceil(e/4)*4,e=Math.max(e,4);let t=new Float32Array(e*e*4);t.set(this.boneMatrices);let n=new Ls(t,e,e,Mn,dn);return n.needsUpdate=!0,this.boneMatrices=t,this.boneTexture=n,this}getBoneByName(e){for(let t=0,n=this.bones.length;t<n;t++){let i=this.bones[t];if(i.name===e)return i}}dispose(){this.boneTexture!==null&&(this.boneTexture.dispose(),this.boneTexture=null),this.previousBoneTexture!==null&&(this.previousBoneTexture.dispose(),this.previousBoneTexture=null)}fromJSON(e,t){this.uuid=e.uuid;for(let n=0,i=e.bones.length;n<i;n++){let r=e.bones[n],o=t[r];o===void 0&&(Xe("Skeleton: No bone found with UUID:",r),o=new Ds),this.bones.push(o),this.boneInverses.push(new ze().fromArray(e.boneInverses[n]))}return this.init(),this}toJSON(){let e={metadata:{version:4.7,type:"Skeleton",generator:"Skeleton.toJSON"},bones:[],boneInverses:[]};e.uuid=this.uuid;let t=this.bones,n=this.boneInverses;for(let i=0,r=t.length;i<r;i++){let o=t[i];e.bones.push(o.uuid);let a=n[i];e.boneInverses.push(a.toArray())}return e}},Ns=class extends gt{constructor(e,t,n,i=1){super(e,t,n),this.isInstancedBufferAttribute=!0,this.meshPerAttribute=i}copy(e){return super.copy(e),this.meshPerAttribute=e.meshPerAttribute,this}toJSON(){let e=super.toJSON();return e.meshPerAttribute=this.meshPerAttribute,e.isInstancedBufferAttribute=!0,e}},_s=new ze,Su=new ze,Oo=[],Mu=new Mt,Bp=new ze,ar=new ke,lr=new zn,Sr=class extends ke{constructor(e,t,n){super(e,t),this.isInstancedMesh=!0,this.instanceMatrix=new Ns(new Float32Array(n*16),16),this.instanceColor=null,this.morphTexture=null,this.count=n,this.boundingBox=null,this.boundingSphere=null;for(let i=0;i<n;i++)this.setMatrixAt(i,Bp)}computeBoundingBox(){let e=this.geometry,t=this.count;this.boundingBox===null&&(this.boundingBox=new Mt),e.boundingBox===null&&e.computeBoundingBox(),this.boundingBox.makeEmpty();for(let n=0;n<t;n++)this.getMatrixAt(n,_s),Mu.copy(e.boundingBox).applyMatrix4(_s),this.boundingBox.union(Mu)}computeBoundingSphere(){let e=this.geometry,t=this.count;this.boundingSphere===null&&(this.boundingSphere=new zn),e.boundingSphere===null&&e.computeBoundingSphere(),this.boundingSphere.makeEmpty();for(let n=0;n<t;n++)this.getMatrixAt(n,_s),lr.copy(e.boundingSphere).applyMatrix4(_s),this.boundingSphere.union(lr)}copy(e,t){return super.copy(e,t),this.instanceMatrix.copy(e.instanceMatrix),this.morphTexture=e.morphTexture!==null?e.morphTexture.clone():null,this.instanceColor=e.instanceColor!==null?e.instanceColor.clone():null,this.count=e.count,this.boundingBox=e.boundingBox!==null?e.boundingBox.clone():null,this.boundingSphere=e.boundingSphere!==null?e.boundingSphere.clone():null,this}getColorAt(e,t){return this.instanceColor===null?t.setRGB(1,1,1):t.fromArray(this.instanceColor.array,e*3)}getMatrixAt(e,t){return t.fromArray(this.instanceMatrix.array,e*16)}getMorphAt(e,t){let n=t.morphTargetInfluences,i=this.morphTexture.source.data.data,r=n.length+1,o=e*r+1;for(let a=0;a<n.length;a++)n[a]=i[o+a]}raycast(e,t){let n=this.matrixWorld,i=this.count;if(ar.geometry=this.geometry,ar.material=this.material,ar.material!==void 0&&(this.boundingSphere===null&&this.computeBoundingSphere(),lr.copy(this.boundingSphere),lr.applyMatrix4(n),e.ray.intersectsSphere(lr)!==!1))for(let r=0;r<i;r++){this.getMatrixAt(r,_s),Su.multiplyMatrices(n,_s),ar.matrixWorld=Su,ar.raycast(e,Oo);for(let o=0,a=Oo.length;o<a;o++){let l=Oo[o];l.instanceId=r,l.object=this,t.push(l)}Oo.length=0}}setColorAt(e,t){return this.instanceColor===null&&(this.instanceColor=new Ns(new Float32Array(this.instanceMatrix.count*3).fill(1),3)),t.toArray(this.instanceColor.array,e*3),this}setMatrixAt(e,t){return t.toArray(this.instanceMatrix.array,e*16),this}setMorphAt(e,t){let n=t.morphTargetInfluences,i=n.length+1;this.morphTexture===null&&(this.morphTexture=new Ls(new Float32Array(i*this.count),i,this.count,Ea,dn));let r=this.morphTexture.source.data.data,o=0;for(let c=0;c<n.length;c++)o+=n[c];let a=this.geometry.morphTargetsRelative?1:1-o,l=i*e;return r[l]=a,r.set(n,l+1),this}updateMorphTargets(){}dispose(){super.dispose(),this.instanceMatrix.dispose(),this.instanceColor!==null&&this.instanceColor.dispose(),this.morphTexture!==null&&(this.morphTexture.dispose(),this.morphTexture=null)}},ki=new zn,zp=new pe(.5,.5),Bo=new L,wi=class{constructor(e=new tn,t=new tn,n=new tn,i=new tn,r=new tn,o=new tn){this.planes=[e,t,n,i,r,o]}set(e,t,n,i,r,o){let a=this.planes;return a[0].copy(e),a[1].copy(t),a[2].copy(n),a[3].copy(i),a[4].copy(r),a[5].copy(o),this}copy(e){let t=this.planes;for(let n=0;n<6;n++)t[n].copy(e.planes[n]);return this}setFromProjectionMatrix(e,t=xn,n=!1){let i=this.planes,r=e.elements,o=r[0],a=r[1],l=r[2],c=r[3],u=r[4],h=r[5],d=r[6],f=r[7],m=r[8],_=r[9],p=r[10],g=r[11],S=r[12],v=r[13],x=r[14],b=r[15];if(i[0].setComponents(c-o,f-u,g-m,b-S).normalize(),i[1].setComponents(c+o,f+u,g+m,b+S).normalize(),i[2].setComponents(c+a,f+h,g+_,b+v).normalize(),i[3].setComponents(c-a,f-h,g-_,b-v).normalize(),n)i[4].setComponents(l,d,p,x).normalize(),i[5].setComponents(c-l,f-d,g-p,b-x).normalize();else if(i[4].setComponents(c-l,f-d,g-p,b-x).normalize(),t===xn)i[5].setComponents(c+l,f+d,g+p,b+x).normalize();else if(t===As)i[5].setComponents(l,d,p,x).normalize();else throw new Error("THREE.Frustum.setFromProjectionMatrix(): Invalid coordinate system: "+t);return this}intersectsObject(e){if(e.boundingSphere!==void 0)e.boundingSphere===null&&e.computeBoundingSphere(),ki.copy(e.boundingSphere).applyMatrix4(e.matrixWorld);else{let t=e.geometry;t.boundingSphere===null&&t.computeBoundingSphere(),ki.copy(t.boundingSphere).applyMatrix4(e.matrixWorld)}return this.intersectsSphere(ki)}intersectsSprite(e){ki.center.set(0,0,0);let t=zp.distanceTo(e.center);return ki.radius=.7071067811865476+t,ki.applyMatrix4(e.matrixWorld),this.intersectsSphere(ki)}intersectsSphere(e){let t=this.planes,n=e.center,i=-e.radius;for(let r=0;r<6;r++)if(t[r].distanceToPoint(n)<i)return!1;return!0}intersectsBox(e){let t=this.planes;for(let n=0;n<6;n++){let i=t[n];if(Bo.x=i.normal.x>0?e.max.x:e.min.x,Bo.y=i.normal.y>0?e.max.y:e.min.y,Bo.z=i.normal.z>0?e.max.z:e.min.z,i.distanceToPoint(Bo)<0)return!1}return!0}containsPoint(e){let t=this.planes;for(let n=0;n<6;n++)if(t[n].distanceToPoint(e)<0)return!1;return!0}clone(){return new this.constructor().copy(this)}},bu=new ze,na=class s{constructor(){this.coordinateSystem=xn,this._frustums=[],this._count=0}setFromArrayCamera(e){let t=e.cameras,n=this._frustums;for(let i=0;i<t.length;i++){let r=t[i];bu.multiplyMatrices(r.projectionMatrix,r.matrixWorldInverse),n[i]===void 0&&(n[i]=new wi),n[i].setFromProjectionMatrix(bu,r.coordinateSystem,r.reversedDepth)}return this._count=t.length,this}getFrustum(e){return this._frustums[e]}intersectsObject(e){let t=this._frustums;for(let n=0;n<this._count;n++)if(t[n].intersectsObject(e))return!0;return!1}intersectsSprite(e){let t=this._frustums;for(let n=0;n<this._count;n++)if(t[n].intersectsSprite(e))return!0;return!1}intersectsSphere(e){let t=this._frustums;for(let n=0;n<this._count;n++)if(t[n].intersectsSphere(e))return!0;return!1}intersectsBox(e){let t=this._frustums;for(let n=0;n<this._count;n++)if(t[n].intersectsBox(e))return!0;return!1}containsPoint(e){let t=this._frustums;for(let n=0;n<this._count;n++)if(t[n].containsPoint(e))return!0;return!1}copy(e){this.coordinateSystem=e.coordinateSystem;let t=this._frustums,n=e._frustums;for(let i=0;i<e._count;i++)t[i]===void 0&&(t[i]=new wi),t[i].copy(n[i]);return this._count=e._count,this}clone(){return new s().copy(this)}};var Mr=class extends Nt{constructor(e=[],t=ji,n,i,r,o,a,l,c,u){super(e,t,n,i,r,o,a,l,c,u),this.isCubeTexture=!0,this.flipY=!1}get images(){return this.image}set images(e){this.image=e}},Hi=class extends Nt{constructor(e,t,n,i,r,o,a,l,c){super(e,t,n,i,r,o,a,l,c),this.isCanvasTexture=!0,this.needsUpdate=!0}};var oi=class extends Nt{constructor(e,t,n=Pn,i,r,o,a=Vt,l=Vt,c,u=ii,h=1){if(u!==ii&&u!==di)throw new Error("THREE.DepthTexture: format must be either THREE.DepthFormat or THREE.DepthStencilFormat");let d={width:e,height:t,depth:h};super(d,i,r,o,a,l,u,n,c),this.isDepthTexture=!0,this.flipY=!1,this.generateMipmaps=!1,this.compareFunction=null}copy(e){return super.copy(e),this.source=new Rs(Object.assign({},e.image)),this.compareFunction=e.compareFunction,this}toJSON(e){let t=super.toJSON(e);return t.compareFunction=this.compareFunction,t}},br=class extends oi{constructor(e,t=Pn,n=ji,i,r,o=Vt,a=Vt,l,c=ii){let u={width:e,height:e,depth:1},h=[u,u,u,u,u,u];super(e,e,t,n,i,r,o,a,l,c),this.image=h,this.isCubeDepthTexture=!0,this.isCubeTexture=!0}get images(){return this.image}set images(e){this.image=e}},wr=class extends Nt{constructor(e=null){super(),this.sourceTexture=e,this.isExternalTexture=!0}copy(e){return super.copy(e),this.sourceTexture=e.sourceTexture,this}},sn=class s extends Lt{constructor(e=1,t=1,n=1,i=1,r=1,o=1){super(),this.type="BoxGeometry",this.parameters={width:e,height:t,depth:n,widthSegments:i,heightSegments:r,depthSegments:o};let a=this;i=Math.floor(i),r=Math.floor(r),o=Math.floor(o);let l=[],c=[],u=[],h=[],d=0,f=0;m("z","y","x",-1,-1,n,t,e,o,r,0),m("z","y","x",1,-1,n,t,-e,o,r,1),m("x","z","y",1,1,e,n,t,i,o,2),m("x","z","y",1,-1,e,n,-t,i,o,3),m("x","y","z",1,-1,e,t,n,i,r,4),m("x","y","z",-1,-1,e,t,-n,i,r,5),this.setIndex(l),this.setAttribute("position",new ct(c,3)),this.setAttribute("normal",new ct(u,3)),this.setAttribute("uv",new ct(h,2));function m(_,p,g,S,v,x,b,w,y,M,C){let E=x/y,R=b/M,D=x/2,k=b/2,P=w/2,O=y+1,G=M+1,U=0,ee=0,W=new L;for(let K=0;K<G;K++){let $=K*R-k;for(let he=0;he<O;he++){let de=he*E-D;W[_]=de*S,W[p]=$*v,W[g]=P,c.push(W.x,W.y,W.z),W[_]=0,W[p]=0,W[g]=w>0?1:-1,u.push(W.x,W.y,W.z),h.push(he/y),h.push(1-K/M),U+=1}}for(let K=0;K<M;K++)for(let $=0;$<y;$++){let he=d+$+O*K,de=d+$+O*(K+1),q=d+($+1)+O*(K+1),ie=d+($+1)+O*K;l.push(he,de,ie),l.push(de,q,ie),ee+=6}a.addGroup(f,ee,C),f+=ee,d+=U}}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}static fromJSON(e){return new s(e.width,e.height,e.depth,e.widthSegments,e.heightSegments,e.depthSegments)}},Ar=class s extends Lt{constructor(e=1,t=1,n=4,i=8,r=1){super(),this.type="CapsuleGeometry",this.parameters={radius:e,height:t,capSegments:n,radialSegments:i,heightSegments:r},t=Math.max(0,t),n=Math.max(1,Math.floor(n)),i=Math.max(3,Math.floor(i)),r=Math.max(1,Math.floor(r));let o=[],a=[],l=[],c=[],u=t/2,h=Math.PI/2*e,d=t,f=2*h+d,m=n*2+r,_=i+1,p=new L,g=new L;for(let S=0;S<=m;S++){let v=0,x=0,b=0,w=0;if(S<=n){let C=S/n,E=C*Math.PI/2;x=-u-e*Math.cos(E),b=e*Math.sin(E),w=-e*Math.cos(E),v=C*h}else if(S<=n+r){let C=(S-n)/r;x=-u+C*t,b=e,w=0,v=h+C*d}else{let C=(S-n-r)/n,E=C*Math.PI/2;x=u+e*Math.sin(E),b=e*Math.cos(E),w=e*Math.sin(E),v=h+d+C*h}let y=Math.max(0,Math.min(1,v/f)),M=0;S===0?M=.5/i:S===m&&(M=-.5/i);for(let C=0;C<=i;C++){let E=C/i,R=E*Math.PI*2,D=Math.sin(R),k=Math.cos(R);g.x=-b*k,g.y=x,g.z=b*D,a.push(g.x,g.y,g.z),p.set(-b*k,w,b*D),p.normalize(),l.push(p.x,p.y,p.z),c.push(E+M,y)}if(S>0){let C=(S-1)*_;for(let E=0;E<i;E++){let R=C+E,D=C+E+1,k=S*_+E,P=S*_+E+1;o.push(R,D,k),o.push(D,P,k)}}}this.setIndex(o),this.setAttribute("position",new ct(a,3)),this.setAttribute("normal",new ct(l,3)),this.setAttribute("uv",new ct(c,2))}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}static fromJSON(e){return new s(e.radius,e.height,e.capSegments,e.radialSegments,e.heightSegments)}},ai=class s extends Lt{constructor(e=1,t=32,n=0,i=Math.PI*2){super(),this.type="CircleGeometry",this.parameters={radius:e,segments:t,thetaStart:n,thetaLength:i},t=Math.max(3,t);let r=[],o=[],a=[],l=[],c=new L,u=new pe;o.push(0,0,0),a.push(0,0,1),l.push(.5,.5);for(let h=0,d=3;h<=t;h++,d+=3){let f=n+h/t*i;c.x=e*Math.cos(f),c.y=e*Math.sin(f),o.push(c.x,c.y,c.z),a.push(0,0,1),u.x=(o[d]/e+1)/2,u.y=(o[d+1]/e+1)/2,l.push(u.x,u.y)}for(let h=1;h<=t;h++)r.push(h,h+1,0);this.setIndex(r),this.setAttribute("position",new ct(o,3)),this.setAttribute("normal",new ct(a,3)),this.setAttribute("uv",new ct(l,2))}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}static fromJSON(e){return new s(e.radius,e.segments,e.thetaStart,e.thetaLength)}},Rn=class s extends Lt{constructor(e=1,t=1,n=1,i=32,r=1,o=!1,a=0,l=Math.PI*2){super(),this.type="CylinderGeometry",this.parameters={radiusTop:e,radiusBottom:t,height:n,radialSegments:i,heightSegments:r,openEnded:o,thetaStart:a,thetaLength:l};let c=this;i=Math.floor(i),r=Math.floor(r);let u=[],h=[],d=[],f=[],m=0,_=[],p=n/2,g=0;S(),o===!1&&(e>0&&v(!0),t>0&&v(!1)),this.setIndex(u),this.setAttribute("position",new ct(h,3)),this.setAttribute("normal",new ct(d,3)),this.setAttribute("uv",new ct(f,2));function S(){let x=new L,b=new L,w=0,y=(t-e)/n;for(let M=0;M<=r;M++){let C=[],E=M/r,R=E*(t-e)+e;for(let D=0;D<=i;D++){let k=D/i,P=k*l+a,O=Math.sin(P),G=Math.cos(P);b.x=R*O,b.y=-E*n+p,b.z=R*G,h.push(b.x,b.y,b.z),x.set(O,y,G).normalize(),d.push(x.x,x.y,x.z),f.push(k,1-E),C.push(m++)}_.push(C)}for(let M=0;M<i;M++)for(let C=0;C<r;C++){let E=_[C][M],R=_[C+1][M],D=_[C+1][M+1],k=_[C][M+1];(e>0||C!==0)&&(u.push(E,R,k),w+=3),(t>0||C!==r-1)&&(u.push(R,D,k),w+=3)}c.addGroup(g,w,0),g+=w}function v(x){let b=m,w=new pe,y=new L,M=0,C=x===!0?e:t,E=x===!0?1:-1;for(let D=1;D<=i;D++)h.push(0,p*E,0),d.push(0,E,0),f.push(.5,.5),m++;let R=m;for(let D=0;D<=i;D++){let P=D/i*l+a,O=Math.cos(P),G=Math.sin(P);y.x=C*G,y.y=p*E,y.z=C*O,h.push(y.x,y.y,y.z),d.push(0,E,0),w.x=O*.5+.5,w.y=G*.5*E+.5,f.push(w.x,w.y),m++}for(let D=0;D<i;D++){let k=b+D,P=R+D;x===!0?u.push(P,P+1,k):u.push(P+1,P,k),M+=3}c.addGroup(g,M,x===!0?1:2),g+=M}}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}static fromJSON(e){return new s(e.radiusTop,e.radiusBottom,e.height,e.radialSegments,e.heightSegments,e.openEnded,e.thetaStart,e.thetaLength)}},Tr=class s extends Rn{constructor(e=1,t=1,n=32,i=1,r=!1,o=0,a=Math.PI*2){super(0,e,t,n,i,r,o,a),this.type="ConeGeometry",this.parameters={radius:e,height:t,radialSegments:n,heightSegments:i,openEnded:r,thetaStart:o,thetaLength:a}}static fromJSON(e){return new s(e.radius,e.height,e.radialSegments,e.heightSegments,e.openEnded,e.thetaStart,e.thetaLength)}};var un=class{constructor(){this.type="Curve",this.arcLengthDivisions=200,this.needsUpdate=!1,this.cacheArcLengths=null}getPoint(){Xe("Curve: .getPoint() not implemented.")}getPointAt(e,t){let n=this.getUtoTmapping(e);return this.getPoint(n,t)}getPoints(e=5){let t=[];for(let n=0;n<=e;n++)t.push(this.getPoint(n/e));return t}getSpacedPoints(e=5){let t=[];for(let n=0;n<=e;n++)t.push(this.getPointAt(n/e));return t}getLength(){let e=this.getLengths();return e[e.length-1]}getLengths(e=this.arcLengthDivisions){if(this.cacheArcLengths&&this.cacheArcLengths.length===e+1&&!this.needsUpdate)return this.cacheArcLengths;this.needsUpdate=!1;let t=[],n,i=this.getPoint(0),r=0;t.push(0);for(let o=1;o<=e;o++)n=this.getPoint(o/e),r+=n.distanceTo(i),t.push(r),i=n;return this.cacheArcLengths=t,t}updateArcLengths(){this.needsUpdate=!0,this.getLengths()}getUtoTmapping(e,t=null){let n=this.getLengths(),i=0,r=n.length,o;t?o=t:o=e*n[r-1];let a=0,l=r-1,c;for(;a<=l;)if(i=Math.floor(a+(l-a)/2),c=n[i]-o,c<0)a=i+1;else if(c>0)l=i-1;else{l=i;break}if(i=l,n[i]===o)return i/(r-1);let u=n[i],d=n[i+1]-u,f=(o-u)/d;return(i+f)/(r-1)}getTangent(e,t){let i=e-1e-4,r=e+1e-4;i<0&&(i=0),r>1&&(r=1);let o=this.getPoint(i),a=this.getPoint(r),l=t||(o.isVector2?new pe:new L);return l.copy(a).sub(o).normalize(),l}getTangentAt(e,t){let n=this.getUtoTmapping(e);return this.getTangent(n,t)}computeFrenetFrames(e,t=!1){let n=new L,i=[],r=[],o=[],a=new L,l=new ze;for(let f=0;f<=e;f++){let m=f/e;i[f]=this.getTangentAt(m,new L)}r[0]=new L,o[0]=new L;let c=Number.MAX_VALUE,u=Math.abs(i[0].x),h=Math.abs(i[0].y),d=Math.abs(i[0].z);u<=c&&(c=u,n.set(1,0,0)),h<=c&&(c=h,n.set(0,1,0)),d<=c&&n.set(0,0,1),a.crossVectors(i[0],n).normalize(),r[0].crossVectors(i[0],a),o[0].crossVectors(i[0],r[0]);for(let f=1;f<=e;f++){if(r[f]=r[f-1].clone(),o[f]=o[f-1].clone(),a.crossVectors(i[f-1],i[f]),a.length()>Number.EPSILON){a.normalize();let m=Math.acos(it(i[f-1].dot(i[f]),-1,1));r[f].applyMatrix4(l.makeRotationAxis(a,m))}o[f].crossVectors(i[f],r[f])}if(t===!0){let f=Math.acos(it(r[0].dot(r[e]),-1,1));f/=e,i[0].dot(a.crossVectors(r[0],r[e]))>0&&(f=-f);for(let m=1;m<=e;m++)r[m].applyMatrix4(l.makeRotationAxis(i[m],f*m)),o[m].crossVectors(i[m],r[m])}return{tangents:i,normals:r,binormals:o}}clone(){return new this.constructor().copy(this)}copy(e){return this.arcLengthDivisions=e.arcLengthDivisions,this.cacheArcLengths=null,this}toJSON(){let e={metadata:{version:4.7,type:"Curve",generator:"Curve.toJSON"}};return e.arcLengthDivisions=this.arcLengthDivisions,e.type=this.type,e}fromJSON(e){return this.arcLengthDivisions=e.arcLengthDivisions,this.cacheArcLengths=null,this}},Us=class extends un{constructor(e=0,t=0,n=1,i=1,r=0,o=Math.PI*2,a=!1,l=0){super(),this.isEllipseCurve=!0,this.type="EllipseCurve",this.aX=e,this.aY=t,this.xRadius=n,this.yRadius=i,this.aStartAngle=r,this.aEndAngle=o,this.aClockwise=a,this.aRotation=l}getPoint(e,t=new pe){let n=t,i=Math.PI*2,r=this.aEndAngle-this.aStartAngle,o=Math.abs(r)<Number.EPSILON;for(;r<0;)r+=i;for(;r>i;)r-=i;r<Number.EPSILON&&(o?r=0:r=i),this.aClockwise===!0&&!o&&(r===i?r=-i:r=r-i);let a=this.aStartAngle+e*r,l=this.aX+this.xRadius*Math.cos(a),c=this.aY+this.yRadius*Math.sin(a);if(this.aRotation!==0){let u=Math.cos(this.aRotation),h=Math.sin(this.aRotation),d=l-this.aX,f=c-this.aY;l=d*u-f*h+this.aX,c=d*h+f*u+this.aY}return n.set(l,c)}copy(e){return super.copy(e),this.aX=e.aX,this.aY=e.aY,this.xRadius=e.xRadius,this.yRadius=e.yRadius,this.aStartAngle=e.aStartAngle,this.aEndAngle=e.aEndAngle,this.aClockwise=e.aClockwise,this.aRotation=e.aRotation,this}toJSON(){let e=super.toJSON();return e.aX=this.aX,e.aY=this.aY,e.xRadius=this.xRadius,e.yRadius=this.yRadius,e.aStartAngle=this.aStartAngle,e.aEndAngle=this.aEndAngle,e.aClockwise=this.aClockwise,e.aRotation=this.aRotation,e}fromJSON(e){return super.fromJSON(e),this.aX=e.aX,this.aY=e.aY,this.xRadius=e.xRadius,this.yRadius=e.yRadius,this.aStartAngle=e.aStartAngle,this.aEndAngle=e.aEndAngle,this.aClockwise=e.aClockwise,this.aRotation=e.aRotation,this}},ia=class extends Us{constructor(e,t,n,i,r,o){super(e,t,n,n,i,r,o),this.isArcCurve=!0,this.type="ArcCurve"}};function Jc(){let s=0,e=0,t=0,n=0;function i(r,o,a,l){s=r,e=a,t=-3*r+3*o-2*a-l,n=2*r-2*o+a+l}return{initCatmullRom:function(r,o,a,l,c){i(o,a,c*(a-r),c*(l-o))},initNonuniformCatmullRom:function(r,o,a,l,c,u,h){let d=(o-r)/c-(a-r)/(c+u)+(a-o)/u,f=(a-o)/u-(l-o)/(u+h)+(l-a)/h;d*=u,f*=u,i(o,a,d,f)},calc:function(r){let o=r*r,a=o*r;return s+e*r+t*o+n*a}}}var wu=new L,Au=new L,ic=new Jc,sc=new Jc,rc=new Jc,sa=class extends un{constructor(e=[],t=!1,n="centripetal",i=.5){super(),this.isCatmullRomCurve3=!0,this.type="CatmullRomCurve3",this.points=e,this.closed=t,this.curveType=n,this.tension=i}getPoint(e,t=new L){let n=t,i=this.points,r=i.length,o=(r-(this.closed?0:1))*e,a=Math.floor(o),l=o-a;this.closed?a+=a>0?0:(Math.floor(Math.abs(a)/r)+1)*r:l===0&&a===r-1&&(a=r-2,l=1);let c,u;this.closed||a>0?c=i[(a-1)%r]:(Au.subVectors(i[0],i[1]).add(i[0]),c=Au);let h=i[a%r],d=i[(a+1)%r];if(this.closed||a+2<r?u=i[(a+2)%r]:(wu.subVectors(i[r-1],i[r-2]).add(i[r-1]),u=wu),this.curveType==="centripetal"||this.curveType==="chordal"){let f=this.curveType==="chordal"?.5:.25,m=Math.pow(c.distanceToSquared(h),f),_=Math.pow(h.distanceToSquared(d),f),p=Math.pow(d.distanceToSquared(u),f);_<1e-4&&(_=1),m<1e-4&&(m=_),p<1e-4&&(p=_),ic.initNonuniformCatmullRom(c.x,h.x,d.x,u.x,m,_,p),sc.initNonuniformCatmullRom(c.y,h.y,d.y,u.y,m,_,p),rc.initNonuniformCatmullRom(c.z,h.z,d.z,u.z,m,_,p)}else this.curveType==="catmullrom"&&(ic.initCatmullRom(c.x,h.x,d.x,u.x,this.tension),sc.initCatmullRom(c.y,h.y,d.y,u.y,this.tension),rc.initCatmullRom(c.z,h.z,d.z,u.z,this.tension));return n.set(ic.calc(l),sc.calc(l),rc.calc(l)),n}copy(e){super.copy(e),this.points=[];for(let t=0,n=e.points.length;t<n;t++){let i=e.points[t];this.points.push(i.clone())}return this.closed=e.closed,this.curveType=e.curveType,this.tension=e.tension,this}toJSON(){let e=super.toJSON();e.points=[];for(let t=0,n=this.points.length;t<n;t++){let i=this.points[t];e.points.push(i.toArray())}return e.closed=this.closed,e.curveType=this.curveType,e.tension=this.tension,e}fromJSON(e){super.fromJSON(e),this.points=[];for(let t=0,n=e.points.length;t<n;t++){let i=e.points[t];this.points.push(new L().fromArray(i))}return this.closed=e.closed,this.curveType=e.curveType,this.tension=e.tension,this}};function Tu(s,e,t,n,i){let r=(n-e)*.5,o=(i-t)*.5,a=s*s,l=s*a;return(2*t-2*n+r+o)*l+(-3*t+3*n-2*r-o)*a+r*s+t}function kp(s,e){let t=1-s;return t*t*e}function Vp(s,e){return 2*(1-s)*s*e}function Gp(s,e){return s*s*e}function ur(s,e,t,n){return kp(s,e)+Vp(s,t)+Gp(s,n)}function Hp(s,e){let t=1-s;return t*t*t*e}function Wp(s,e){let t=1-s;return 3*t*t*s*e}function Xp(s,e){return 3*(1-s)*s*s*e}function qp(s,e){return s*s*s*e}function fr(s,e,t,n,i){return Hp(s,e)+Wp(s,t)+Xp(s,n)+qp(s,i)}var Er=class extends un{constructor(e=new pe,t=new pe,n=new pe,i=new pe){super(),this.isCubicBezierCurve=!0,this.type="CubicBezierCurve",this.v0=e,this.v1=t,this.v2=n,this.v3=i}getPoint(e,t=new pe){let n=t,i=this.v0,r=this.v1,o=this.v2,a=this.v3;return n.set(fr(e,i.x,r.x,o.x,a.x),fr(e,i.y,r.y,o.y,a.y)),n}copy(e){return super.copy(e),this.v0.copy(e.v0),this.v1.copy(e.v1),this.v2.copy(e.v2),this.v3.copy(e.v3),this}toJSON(){let e=super.toJSON();return e.v0=this.v0.toArray(),e.v1=this.v1.toArray(),e.v2=this.v2.toArray(),e.v3=this.v3.toArray(),e}fromJSON(e){return super.fromJSON(e),this.v0.fromArray(e.v0),this.v1.fromArray(e.v1),this.v2.fromArray(e.v2),this.v3.fromArray(e.v3),this}},ra=class extends un{constructor(e=new L,t=new L,n=new L,i=new L){super(),this.isCubicBezierCurve3=!0,this.type="CubicBezierCurve3",this.v0=e,this.v1=t,this.v2=n,this.v3=i}getPoint(e,t=new L){let n=t,i=this.v0,r=this.v1,o=this.v2,a=this.v3;return n.set(fr(e,i.x,r.x,o.x,a.x),fr(e,i.y,r.y,o.y,a.y),fr(e,i.z,r.z,o.z,a.z)),n}copy(e){return super.copy(e),this.v0.copy(e.v0),this.v1.copy(e.v1),this.v2.copy(e.v2),this.v3.copy(e.v3),this}toJSON(){let e=super.toJSON();return e.v0=this.v0.toArray(),e.v1=this.v1.toArray(),e.v2=this.v2.toArray(),e.v3=this.v3.toArray(),e}fromJSON(e){return super.fromJSON(e),this.v0.fromArray(e.v0),this.v1.fromArray(e.v1),this.v2.fromArray(e.v2),this.v3.fromArray(e.v3),this}},Rr=class extends un{constructor(e=new pe,t=new pe){super(),this.isLineCurve=!0,this.type="LineCurve",this.v1=e,this.v2=t}getPoint(e,t=new pe){let n=t;return e===1?n.copy(this.v2):(n.copy(this.v2).sub(this.v1),n.multiplyScalar(e).add(this.v1)),n}getPointAt(e,t){return this.getPoint(e,t)}getTangent(e,t=new pe){return t.subVectors(this.v2,this.v1).normalize()}getTangentAt(e,t){return this.getTangent(e,t)}copy(e){return super.copy(e),this.v1.copy(e.v1),this.v2.copy(e.v2),this}toJSON(){let e=super.toJSON();return e.v1=this.v1.toArray(),e.v2=this.v2.toArray(),e}fromJSON(e){return super.fromJSON(e),this.v1.fromArray(e.v1),this.v2.fromArray(e.v2),this}},oa=class extends un{constructor(e=new L,t=new L){super(),this.isLineCurve3=!0,this.type="LineCurve3",this.v1=e,this.v2=t}getPoint(e,t=new L){let n=t;return e===1?n.copy(this.v2):(n.copy(this.v2).sub(this.v1),n.multiplyScalar(e).add(this.v1)),n}getPointAt(e,t){return this.getPoint(e,t)}getTangent(e,t=new L){return t.subVectors(this.v2,this.v1).normalize()}getTangentAt(e,t){return this.getTangent(e,t)}copy(e){return super.copy(e),this.v1.copy(e.v1),this.v2.copy(e.v2),this}toJSON(){let e=super.toJSON();return e.v1=this.v1.toArray(),e.v2=this.v2.toArray(),e}fromJSON(e){return super.fromJSON(e),this.v1.fromArray(e.v1),this.v2.fromArray(e.v2),this}},Cr=class extends un{constructor(e=new pe,t=new pe,n=new pe){super(),this.isQuadraticBezierCurve=!0,this.type="QuadraticBezierCurve",this.v0=e,this.v1=t,this.v2=n}getPoint(e,t=new pe){let n=t,i=this.v0,r=this.v1,o=this.v2;return n.set(ur(e,i.x,r.x,o.x),ur(e,i.y,r.y,o.y)),n}copy(e){return super.copy(e),this.v0.copy(e.v0),this.v1.copy(e.v1),this.v2.copy(e.v2),this}toJSON(){let e=super.toJSON();return e.v0=this.v0.toArray(),e.v1=this.v1.toArray(),e.v2=this.v2.toArray(),e}fromJSON(e){return super.fromJSON(e),this.v0.fromArray(e.v0),this.v1.fromArray(e.v1),this.v2.fromArray(e.v2),this}},aa=class extends un{constructor(e=new L,t=new L,n=new L){super(),this.isQuadraticBezierCurve3=!0,this.type="QuadraticBezierCurve3",this.v0=e,this.v1=t,this.v2=n}getPoint(e,t=new L){let n=t,i=this.v0,r=this.v1,o=this.v2;return n.set(ur(e,i.x,r.x,o.x),ur(e,i.y,r.y,o.y),ur(e,i.z,r.z,o.z)),n}copy(e){return super.copy(e),this.v0.copy(e.v0),this.v1.copy(e.v1),this.v2.copy(e.v2),this}toJSON(){let e=super.toJSON();return e.v0=this.v0.toArray(),e.v1=this.v1.toArray(),e.v2=this.v2.toArray(),e}fromJSON(e){return super.fromJSON(e),this.v0.fromArray(e.v0),this.v1.fromArray(e.v1),this.v2.fromArray(e.v2),this}},Pr=class extends un{constructor(e=[]){super(),this.isSplineCurve=!0,this.type="SplineCurve",this.points=e}getPoint(e,t=new pe){let n=t,i=this.points,r=(i.length-1)*e,o=Math.floor(r),a=r-o,l=i[o===0?o:o-1],c=i[o],u=i[o>i.length-2?i.length-1:o+1],h=i[o>i.length-3?i.length-1:o+2];return n.set(Tu(a,l.x,c.x,u.x,h.x),Tu(a,l.y,c.y,u.y,h.y)),n}copy(e){super.copy(e),this.points=[];for(let t=0,n=e.points.length;t<n;t++){let i=e.points[t];this.points.push(i.clone())}return this}toJSON(){let e=super.toJSON();e.points=[];for(let t=0,n=this.points.length;t<n;t++){let i=this.points[t];e.points.push(i.toArray())}return e}fromJSON(e){super.fromJSON(e),this.points=[];for(let t=0,n=e.points.length;t<n;t++){let i=e.points[t];this.points.push(new pe().fromArray(i))}return this}},fc=Object.freeze({__proto__:null,ArcCurve:ia,CatmullRomCurve3:sa,CubicBezierCurve:Er,CubicBezierCurve3:ra,EllipseCurve:Us,LineCurve:Rr,LineCurve3:oa,QuadraticBezierCurve:Cr,QuadraticBezierCurve3:aa,SplineCurve:Pr}),la=class extends un{constructor(){super(),this.type="CurvePath",this.curves=[],this.autoClose=!1,this.cacheLengths=null}add(e){this.curves.push(e)}closePath(){let e=this.curves[0].getPoint(0),t=this.curves[this.curves.length-1].getPoint(1);if(!e.equals(t)){let n=e.isVector2===!0?"LineCurve":"LineCurve3";this.curves.push(new fc[n](t,e))}return this}getPoint(e,t){let n=e*this.getLength(),i=this.getCurveLengths(),r=0;for(;r<i.length;){if(i[r]>=n){let o=i[r]-n,a=this.curves[r],l=a.getLength(),c=l===0?0:1-o/l;return a.getPointAt(c,t)}r++}return null}getLength(){let e=this.getCurveLengths();return e[e.length-1]}updateArcLengths(){this.needsUpdate=!0,this.cacheLengths=null,this.getCurveLengths()}getCurveLengths(){if(this.cacheLengths&&this.cacheLengths.length===this.curves.length)return this.cacheLengths;let e=[],t=0;for(let n=0,i=this.curves.length;n<i;n++)t+=this.curves[n].getLength(),e.push(t);return this.cacheLengths=e,e}getSpacedPoints(e=40){let t=[];for(let n=0;n<=e;n++)t.push(this.getPoint(n/e));return this.autoClose&&t.push(t[0]),t}getPoints(e=12){let t=[],n;for(let i=0,r=this.curves;i<r.length;i++){let o=r[i],a=o.isEllipseCurve?e*2:o.isLineCurve||o.isLineCurve3?1:o.isSplineCurve?e*o.points.length:e,l=o.getPoints(a);for(let c=0;c<l.length;c++){let u=l[c];n&&n.equals(u)||(t.push(u),n=u)}}return this.autoClose&&t.length>1&&!t[t.length-1].equals(t[0])&&t.push(t[0]),t}copy(e){super.copy(e),this.curves=[];for(let t=0,n=e.curves.length;t<n;t++){let i=e.curves[t];this.curves.push(i.clone())}return this.autoClose=e.autoClose,this.cacheLengths=null,this}toJSON(){let e=super.toJSON();e.autoClose=this.autoClose,e.curves=[];for(let t=0,n=this.curves.length;t<n;t++){let i=this.curves[t];e.curves.push(i.toJSON())}return e}fromJSON(e){super.fromJSON(e),this.autoClose=e.autoClose,this.curves=[];for(let t=0,n=e.curves.length;t<n;t++){let i=e.curves[t];this.curves.push(new fc[i.type]().fromJSON(i))}return this.cacheLengths=null,this}},Ir=class extends la{constructor(e){super(),this.type="Path",this.currentPoint=new pe,e&&this.setFromPoints(e)}setFromPoints(e){this.moveTo(e[0].x,e[0].y);for(let t=1,n=e.length;t<n;t++)this.lineTo(e[t].x,e[t].y);return this}moveTo(e,t){return this.currentPoint.set(e,t),this}lineTo(e,t){let n=new Rr(this.currentPoint.clone(),new pe(e,t));return this.curves.push(n),this.currentPoint.set(e,t),this}quadraticCurveTo(e,t,n,i){let r=new Cr(this.currentPoint.clone(),new pe(e,t),new pe(n,i));return this.curves.push(r),this.currentPoint.set(n,i),this}bezierCurveTo(e,t,n,i,r,o){let a=new Er(this.currentPoint.clone(),new pe(e,t),new pe(n,i),new pe(r,o));return this.curves.push(a),this.currentPoint.set(r,o),this}splineThru(e){let t=[this.currentPoint.clone()].concat(e),n=new Pr(t);return this.curves.push(n),this.currentPoint.copy(e[e.length-1]),this}arc(e,t,n,i,r,o){let a=this.currentPoint.x,l=this.currentPoint.y;return this.absarc(e+a,t+l,n,i,r,o),this}absarc(e,t,n,i,r,o){return this.absellipse(e,t,n,n,i,r,o),this}ellipse(e,t,n,i,r,o,a,l){let c=this.currentPoint.x,u=this.currentPoint.y;return this.absellipse(e+c,t+u,n,i,r,o,a,l),this}absellipse(e,t,n,i,r,o,a,l){let c=new Us(e,t,n,i,r,o,a,l);if(this.curves.length>0){let h=c.getPoint(0);h.equals(this.currentPoint)||this.lineTo(h.x,h.y)}this.curves.push(c);let u=c.getPoint(1);return this.currentPoint.copy(u),this}copy(e){return super.copy(e),this.currentPoint.copy(e.currentPoint),this}toJSON(){let e=super.toJSON();return e.currentPoint=this.currentPoint.toArray(),e}fromJSON(e){return super.fromJSON(e),this.currentPoint.fromArray(e.currentPoint),this}},kn=class extends Ir{constructor(e){super(e),this.uuid=Pi(),this.type="Shape",this.holes=[]}getPointsHoles(e){let t=[];for(let n=0,i=this.holes.length;n<i;n++)t[n]=this.holes[n].getPoints(e);return t}extractPoints(e){return{shape:this.getPoints(e),holes:this.getPointsHoles(e)}}copy(e){super.copy(e),this.holes=[];for(let t=0,n=e.holes.length;t<n;t++){let i=e.holes[t];this.holes.push(i.clone())}return this}toJSON(){let e=super.toJSON();e.uuid=this.uuid,e.holes=[];for(let t=0,n=this.holes.length;t<n;t++){let i=this.holes[t];e.holes.push(i.toJSON())}return e}fromJSON(e){super.fromJSON(e),this.uuid=e.uuid,this.holes=[];for(let t=0,n=e.holes.length;t<n;t++){let i=e.holes[t];this.holes.push(new Ir().fromJSON(i))}return this}};function Yp(s,e,t=2){let n=e&&e.length,i=n?e[0]*t:s.length,r=Cf(s,0,i,t,!0),o=[];if(!r||r.next===r.prev)return o;let a,l,c;if(n&&(r=jp(s,e,r,t)),s.length>80*t){a=s[0],l=s[1];let u=a,h=l;for(let d=t;d<i;d+=t){let f=s[d],m=s[d+1];f<a&&(a=f),m<l&&(l=m),f>u&&(u=f),m>h&&(h=m)}c=Math.max(u-a,h-l),c=c!==0?32767/c:0}return Dr(r,o,t,a,l,c,0),o}function Cf(s,e,t,n,i){let r;if(i===cm(s,e,t,n)>0)for(let o=e;o<t;o+=n)r=Eu(o/n|0,s[o],s[o+1],r);else for(let o=t-n;o>=e;o-=n)r=Eu(o/n|0,s[o],s[o+1],r);return r&&Fs(r,r.next)&&(Nr(r),r=r.next),r}function Wi(s,e){if(!s)return s;e||(e=s);let t=s,n;do if(n=!1,!t.steiner&&(Fs(t,t.next)||Et(t.prev,t,t.next)===0)){if(Nr(t),t=e=t.prev,t===t.next)break;n=!0}else t=t.next;while(n||t!==e);return e}function Dr(s,e,t,n,i,r,o){if(!s)return;!o&&r&&im(s,n,i,r);let a=s;for(;s.prev!==s.next;){let l=s.prev,c=s.next;if(r?Jp(s,n,i,r):Zp(s)){e.push(l.i,s.i,c.i),Nr(s),s=c.next,a=c.next;continue}if(s=c,s===a){o?o===1?(s=$p(Wi(s),e),Dr(s,e,t,n,i,r,2)):o===2&&Kp(s,e,t,n,i,r):Dr(Wi(s),e,t,n,i,r,1);break}}}function Zp(s){let e=s.prev,t=s,n=s.next;if(Et(e,t,n)>=0)return!1;let i=e.x,r=t.x,o=n.x,a=e.y,l=t.y,c=n.y,u=Math.min(i,r,o),h=Math.min(a,l,c),d=Math.max(i,r,o),f=Math.max(a,l,c),m=n.next;for(;m!==e;){if(m.x>=u&&m.x<=d&&m.y>=h&&m.y<=f&&cr(i,a,r,l,o,c,m.x,m.y)&&Et(m.prev,m,m.next)>=0)return!1;m=m.next}return!0}function Jp(s,e,t,n){let i=s.prev,r=s,o=s.next;if(Et(i,r,o)>=0)return!1;let a=i.x,l=r.x,c=o.x,u=i.y,h=r.y,d=o.y,f=Math.min(a,l,c),m=Math.min(u,h,d),_=Math.max(a,l,c),p=Math.max(u,h,d),g=dc(f,m,e,t,n),S=dc(_,p,e,t,n),v=s.prevZ,x=s.nextZ;for(;v&&v.z>=g&&x&&x.z<=S;){if(v.x>=f&&v.x<=_&&v.y>=m&&v.y<=p&&v!==i&&v!==o&&cr(a,u,l,h,c,d,v.x,v.y)&&Et(v.prev,v,v.next)>=0||(v=v.prevZ,x.x>=f&&x.x<=_&&x.y>=m&&x.y<=p&&x!==i&&x!==o&&cr(a,u,l,h,c,d,x.x,x.y)&&Et(x.prev,x,x.next)>=0))return!1;x=x.nextZ}for(;v&&v.z>=g;){if(v.x>=f&&v.x<=_&&v.y>=m&&v.y<=p&&v!==i&&v!==o&&cr(a,u,l,h,c,d,v.x,v.y)&&Et(v.prev,v,v.next)>=0)return!1;v=v.prevZ}for(;x&&x.z<=S;){if(x.x>=f&&x.x<=_&&x.y>=m&&x.y<=p&&x!==i&&x!==o&&cr(a,u,l,h,c,d,x.x,x.y)&&Et(x.prev,x,x.next)>=0)return!1;x=x.nextZ}return!0}function $p(s,e){let t=s;do{let n=t.prev,i=t.next.next;!Fs(n,i)&&If(n,t,t.next,i)&&Lr(n,i)&&Lr(i,n)&&(e.push(n.i,t.i,i.i),Nr(t),Nr(t.next),t=s=i),t=t.next}while(t!==s);return Wi(t)}function Kp(s,e,t,n,i,r){let o=s;do{let a=o.next.next;for(;a!==o.prev;){if(o.i!==a.i&&om(o,a)){let l=Df(o,a);o=Wi(o,o.next),l=Wi(l,l.next),Dr(o,e,t,n,i,r,0),Dr(l,e,t,n,i,r,0);return}a=a.next}o=o.next}while(o!==s)}function jp(s,e,t,n){let i=[];for(let r=0,o=e.length;r<o;r++){let a=e[r]*n,l=r<o-1?e[r+1]*n:s.length,c=Cf(s,a,l,n,!1);c===c.next&&(c.steiner=!0),i.push(rm(c))}i.sort(Qp);for(let r=0;r<i.length;r++)t=em(i[r],t);return t}function Qp(s,e){let t=s.x-e.x;if(t===0&&(t=s.y-e.y,t===0)){let n=(s.next.y-s.y)/(s.next.x-s.x),i=(e.next.y-e.y)/(e.next.x-e.x);t=n-i}return t}function em(s,e){let t=tm(s,e);if(!t)return e;let n=Df(t,s);return Wi(n,n.next),Wi(t,t.next)}function tm(s,e){let t=e,n=s.x,i=s.y,r=-1/0,o;if(Fs(s,t))return t;do{if(Fs(s,t.next))return t.next;if(i<=t.y&&i>=t.next.y&&t.next.y!==t.y){let h=t.x+(i-t.y)*(t.next.x-t.x)/(t.next.y-t.y);if(h<=n&&h>r&&(r=h,o=t.x<t.next.x?t:t.next,h===n))return o}t=t.next}while(t!==e);if(!o)return null;let a=o,l=o.x,c=o.y,u=1/0;t=o;do{if(n>=t.x&&t.x>=l&&n!==t.x&&Pf(i<c?n:r,i,l,c,i<c?r:n,i,t.x,t.y)){let h=Math.abs(i-t.y)/(n-t.x);Lr(t,s)&&(h<u||h===u&&(t.x>o.x||t.x===o.x&&nm(o,t)))&&(o=t,u=h)}t=t.next}while(t!==a);return o}function nm(s,e){return Et(s.prev,s,e.prev)<0&&Et(e.next,s,s.next)<0}function im(s,e,t,n){let i=s;do i.z===0&&(i.z=dc(i.x,i.y,e,t,n)),i.prevZ=i.prev,i.nextZ=i.next,i=i.next;while(i!==s);i.prevZ.nextZ=null,i.prevZ=null,sm(i)}function sm(s){let e,t=1;do{let n=s,i;s=null;let r=null;for(e=0;n;){e++;let o=n,a=0;for(let c=0;c<t&&(a++,o=o.nextZ,!!o);c++);let l=t;for(;a>0||l>0&&o;)a!==0&&(l===0||!o||n.z<=o.z)?(i=n,n=n.nextZ,a--):(i=o,o=o.nextZ,l--),r?r.nextZ=i:s=i,i.prevZ=r,r=i;n=o}r.nextZ=null,t*=2}while(e>1);return s}function dc(s,e,t,n,i){return s=(s-t)*i|0,e=(e-n)*i|0,s=(s|s<<8)&16711935,s=(s|s<<4)&252645135,s=(s|s<<2)&858993459,s=(s|s<<1)&1431655765,e=(e|e<<8)&16711935,e=(e|e<<4)&252645135,e=(e|e<<2)&858993459,e=(e|e<<1)&1431655765,s|e<<1}function rm(s){let e=s,t=s;do(e.x<t.x||e.x===t.x&&e.y<t.y)&&(t=e),e=e.next;while(e!==s);return t}function Pf(s,e,t,n,i,r,o,a){return(i-o)*(e-a)>=(s-o)*(r-a)&&(s-o)*(n-a)>=(t-o)*(e-a)&&(t-o)*(r-a)>=(i-o)*(n-a)}function cr(s,e,t,n,i,r,o,a){return!(s===o&&e===a)&&Pf(s,e,t,n,i,r,o,a)}function om(s,e){return s.next.i!==e.i&&s.prev.i!==e.i&&!am(s,e)&&(Lr(s,e)&&Lr(e,s)&&lm(s,e)&&(Et(s.prev,s,e.prev)||Et(s,e.prev,e))||Fs(s,e)&&Et(s.prev,s,s.next)>0&&Et(e.prev,e,e.next)>0)}function Et(s,e,t){return(e.y-s.y)*(t.x-e.x)-(e.x-s.x)*(t.y-e.y)}function Fs(s,e){return s.x===e.x&&s.y===e.y}function If(s,e,t,n){let i=ko(Et(s,e,t)),r=ko(Et(s,e,n)),o=ko(Et(t,n,s)),a=ko(Et(t,n,e));return!!(i!==r&&o!==a||i===0&&zo(s,t,e)||r===0&&zo(s,n,e)||o===0&&zo(t,s,n)||a===0&&zo(t,e,n))}function zo(s,e,t){return e.x<=Math.max(s.x,t.x)&&e.x>=Math.min(s.x,t.x)&&e.y<=Math.max(s.y,t.y)&&e.y>=Math.min(s.y,t.y)}function ko(s){return s>0?1:s<0?-1:0}function am(s,e){let t=s;do{if(t.i!==s.i&&t.next.i!==s.i&&t.i!==e.i&&t.next.i!==e.i&&If(t,t.next,s,e))return!0;t=t.next}while(t!==s);return!1}function Lr(s,e){return Et(s.prev,s,s.next)<0?Et(s,e,s.next)>=0&&Et(s,s.prev,e)>=0:Et(s,e,s.prev)<0||Et(s,s.next,e)<0}function lm(s,e){let t=s,n=!1,i=(s.x+e.x)/2,r=(s.y+e.y)/2;do t.y>r!=t.next.y>r&&t.next.y!==t.y&&i<(t.next.x-t.x)*(r-t.y)/(t.next.y-t.y)+t.x&&(n=!n),t=t.next;while(t!==s);return n}function Df(s,e){let t=pc(s.i,s.x,s.y),n=pc(e.i,e.x,e.y),i=s.next,r=e.prev;return s.next=e,e.prev=s,t.next=i,i.prev=t,n.next=t,t.prev=n,r.next=n,n.prev=r,n}function Eu(s,e,t,n){let i=pc(s,e,t);return n?(i.next=n.next,i.prev=n,n.next.prev=i,n.next=i):(i.prev=i,i.next=i),i}function Nr(s){s.next.prev=s.prev,s.prev.next=s.next,s.prevZ&&(s.prevZ.nextZ=s.nextZ),s.nextZ&&(s.nextZ.prevZ=s.prevZ)}function pc(s,e,t){return{i:s,x:e,y:t,prev:null,next:null,z:0,prevZ:null,nextZ:null,steiner:!1}}function cm(s,e,t,n){let i=0;for(let r=e,o=t-n;r<t;r+=n)i+=(s[o]-s[r])*(s[r+1]+s[o+1]),o=r;return i}var mc=class{static triangulate(e,t,n=2){return Yp(e,t,n)}},cn=class s{static area(e){let t=e.length,n=0;for(let i=t-1,r=0;r<t;i=r++)n+=e[i].x*e[r].y-e[r].x*e[i].y;return n*.5}static isClockWise(e){return s.area(e)<0}static triangulateShape(e,t){let n=[],i=[],r=[];Ru(e),Cu(n,e);let o=e.length;t.forEach(Ru);for(let l=0;l<t.length;l++)i.push(o),o+=t[l].length,Cu(n,t[l]);let a=mc.triangulate(n,i);for(let l=0;l<a.length;l+=3)r.push(a.slice(l,l+3));return r}};function Ru(s){let e=s.length;e>2&&s[e-1].equals(s[0])&&s.pop()}function Cu(s,e){for(let t=0;t<e.length;t++)s.push(e[t].x),s.push(e[t].y)}var li=class s extends Lt{constructor(e=new kn([new pe(.5,.5),new pe(-.5,.5),new pe(-.5,-.5),new pe(.5,-.5)]),t={}){super(),this.type="ExtrudeGeometry",this.parameters={shapes:e,options:t},e=Array.isArray(e)?e:[e];let n=this,i=[],r=[];for(let a=0,l=e.length;a<l;a++){let c=e[a];o(c)}this.setAttribute("position",new ct(i,3)),this.setAttribute("uv",new ct(r,2)),this.computeVertexNormals();function o(a){let l=[],c=t.curveSegments!==void 0?t.curveSegments:12,u=t.steps!==void 0?t.steps:1,h=t.depth!==void 0?t.depth:1,d=t.bevelEnabled!==void 0?t.bevelEnabled:!0,f=t.bevelThickness!==void 0?t.bevelThickness:.2,m=t.bevelSize!==void 0?t.bevelSize:f-.1,_=t.bevelOffset!==void 0?t.bevelOffset:0,p=t.bevelSegments!==void 0?t.bevelSegments:3,g=t.extrudePath,S=t.UVGenerator!==void 0?t.UVGenerator:hm,v,x=!1,b,w,y,M;if(g){v=g.getSpacedPoints(u),x=!0,d=!1;let N=g.isCatmullRomCurve3?g.closed:!1;b=g.computeFrenetFrames(u,N),w=new L,y=new L,M=new L}d||(p=0,f=0,m=0,_=0);let C=a.extractPoints(c),E=C.shape,R=C.holes;if(!cn.isClockWise(E)){E=E.reverse();for(let N=0,X=R.length;N<X;N++){let j=R[N];cn.isClockWise(j)&&(R[N]=j.reverse())}}function k(N){let j=10000000000000001e-36,ge=N[0];for(let le=1;le<=N.length;le++){let Re=le%N.length,xe=N[Re],Ue=xe.x-ge.x,Je=xe.y-ge.y,Qe=Ue*Ue+Je*Je,B=Math.max(Math.abs(xe.x),Math.abs(xe.y),Math.abs(ge.x),Math.abs(ge.y)),xt=j*B*B;if(Qe<=xt){N.splice(Re,1),le--;continue}ge=xe}}k(E),R.forEach(k);let P=R.length,O=E;for(let N=0;N<P;N++){let X=R[N];E=E.concat(X)}function G(N,X,j){return X||Ye("ExtrudeGeometry: vec does not exist"),N.clone().addScaledVector(X,j)}let U=E.length;function ee(N,X,j){let ge,le,Re,xe=N.x-X.x,Ue=N.y-X.y,Je=j.x-N.x,Qe=j.y-N.y,B=xe*xe+Ue*Ue,xt=xe*Qe-Ue*Je;if(Math.abs(xt)>Number.EPSILON){let je=Math.sqrt(B),rt=Math.sqrt(Je*Je+Qe*Qe),Me=X.x-Ue/je,yt=X.y+xe/je,He=j.x-Qe/rt,qe=j.y+Je/rt,I=((He-Me)*Qe-(qe-yt)*Je)/(xe*Qe-Ue*Je);ge=Me+xe*I-N.x,le=yt+Ue*I-N.y;let A=ge*ge+le*le;if(A<=2)return new pe(ge,le);Re=Math.sqrt(A/2)}else{let je=!1;xe>Number.EPSILON?Je>Number.EPSILON&&(je=!0):xe<-Number.EPSILON?Je<-Number.EPSILON&&(je=!0):Math.sign(Ue)===Math.sign(Qe)&&(je=!0),je?(ge=-Ue,le=xe,Re=Math.sqrt(B)):(ge=xe,le=Ue,Re=Math.sqrt(B/2))}return new pe(ge/Re,le/Re)}let W=[];for(let N=0,X=O.length,j=X-1,ge=N+1;N<X;N++,j++,ge++)j===X&&(j=0),ge===X&&(ge=0),W[N]=ee(O[N],O[j],O[ge]);let K=[],$,he=W.concat();for(let N=0,X=P;N<X;N++){let j=R[N];$=[];for(let ge=0,le=j.length,Re=le-1,xe=ge+1;ge<le;ge++,Re++,xe++)Re===le&&(Re=0),xe===le&&(xe=0),$[ge]=ee(j[ge],j[Re],j[xe]);K.push($),he=he.concat($)}let de;if(p===0)de=cn.triangulateShape(O,R);else{let N=[],X=[];for(let j=0;j<p;j++){let ge=j/p,le=f*Math.cos(ge*Math.PI/2),Re=m*Math.sin(ge*Math.PI/2)+_;for(let xe=0,Ue=O.length;xe<Ue;xe++){let Je=G(O[xe],W[xe],Re);H(Je.x,Je.y,-le),ge===0&&N.push(Je)}for(let xe=0,Ue=P;xe<Ue;xe++){let Je=R[xe];$=K[xe];let Qe=[];for(let B=0,xt=Je.length;B<xt;B++){let je=G(Je[B],$[B],Re);H(je.x,je.y,-le),ge===0&&Qe.push(je)}ge===0&&X.push(Qe)}}de=cn.triangulateShape(N,X)}let q=de.length,ie=m+_;for(let N=0;N<U;N++){let X=d?G(E[N],he[N],ie):E[N];x?(y.copy(b.normals[0]).multiplyScalar(X.x),w.copy(b.binormals[0]).multiplyScalar(X.y),M.copy(v[0]).add(y).add(w),H(M.x,M.y,M.z)):H(X.x,X.y,0)}for(let N=1;N<=u;N++)for(let X=0;X<U;X++){let j=d?G(E[X],he[X],ie):E[X];x?(y.copy(b.normals[N]).multiplyScalar(j.x),w.copy(b.binormals[N]).multiplyScalar(j.y),M.copy(v[N]).add(y).add(w),H(M.x,M.y,M.z)):H(j.x,j.y,h/u*N)}for(let N=p-1;N>=0;N--){let X=N/p,j=f*Math.cos(X*Math.PI/2),ge=m*Math.sin(X*Math.PI/2)+_;for(let le=0,Re=O.length;le<Re;le++){let xe=G(O[le],W[le],ge);H(xe.x,xe.y,h+j)}for(let le=0,Re=R.length;le<Re;le++){let xe=R[le];$=K[le];for(let Ue=0,Je=xe.length;Ue<Je;Ue++){let Qe=G(xe[Ue],$[Ue],ge);x?H(Qe.x,Qe.y+v[u-1].y,v[u-1].x+j):H(Qe.x,Qe.y,h+j)}}}ce(),Ce();function ce(){let N=i.length/3;if(d){let X=0,j=U*X;for(let ge=0;ge<q;ge++){let le=de[ge];se(le[2]+j,le[1]+j,le[0]+j)}X=u+p*2,j=U*X;for(let ge=0;ge<q;ge++){let le=de[ge];se(le[0]+j,le[1]+j,le[2]+j)}}else{for(let X=0;X<q;X++){let j=de[X];se(j[2],j[1],j[0])}for(let X=0;X<q;X++){let j=de[X];se(j[0]+U*u,j[1]+U*u,j[2]+U*u)}}n.addGroup(N,i.length/3-N,0)}function Ce(){let N=i.length/3,X=0;ue(O,X),X+=O.length;for(let j=0,ge=R.length;j<ge;j++){let le=R[j];ue(le,X),X+=le.length}n.addGroup(N,i.length/3-N,1)}function ue(N,X){let j=N.length;for(;--j>=0;){let ge=j,le=j-1;le<0&&(le=N.length-1);for(let Re=0,xe=u+p*2;Re<xe;Re++){let Ue=U*Re,Je=U*(Re+1),Qe=X+ge+Ue,B=X+le+Ue,xt=X+le+Je,je=X+ge+Je;fe(Qe,B,xt,je)}}}function H(N,X,j){l.push(N),l.push(X),l.push(j)}function se(N,X,j){Le(N),Le(X),Le(j);let ge=i.length/3,le=S.generateTopUV(n,i,ge-3,ge-2,ge-1);V(le[0]),V(le[1]),V(le[2])}function fe(N,X,j,ge){Le(N),Le(X),Le(ge),Le(X),Le(j),Le(ge);let le=i.length/3,Re=S.generateSideWallUV(n,i,le-6,le-3,le-2,le-1);V(Re[0]),V(Re[1]),V(Re[3]),V(Re[1]),V(Re[2]),V(Re[3])}function Le(N){i.push(l[N*3+0]),i.push(l[N*3+1]),i.push(l[N*3+2])}function V(N){r.push(N.x),r.push(N.y)}}}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}toJSON(){let e=super.toJSON(),t=this.parameters.shapes,n=this.parameters.options;return um(t,n,e)}static fromJSON(e,t){let n=[];for(let r=0,o=e.shapes.length;r<o;r++){let a=t[e.shapes[r]];n.push(a)}let i=e.options.extrudePath;return i!==void 0&&(e.options.extrudePath=new fc[i.type]().fromJSON(i)),new s(n,e.options)}},hm={generateTopUV:function(s,e,t,n,i){let r=e[t*3],o=e[t*3+1],a=e[n*3],l=e[n*3+1],c=e[i*3],u=e[i*3+1];return[new pe(r,o),new pe(a,l),new pe(c,u)]},generateSideWallUV:function(s,e,t,n,i,r){let o=e[t*3],a=e[t*3+1],l=e[t*3+2],c=e[n*3],u=e[n*3+1],h=e[n*3+2],d=e[i*3],f=e[i*3+1],m=e[i*3+2],_=e[r*3],p=e[r*3+1],g=e[r*3+2];return Math.abs(a-u)<Math.abs(o-c)?[new pe(o,1-l),new pe(c,1-h),new pe(d,1-m),new pe(_,1-g)]:[new pe(a,1-l),new pe(u,1-h),new pe(f,1-m),new pe(p,1-g)]}};function um(s,e,t){if(t.shapes=[],Array.isArray(s))for(let n=0,i=s.length;n<i;n++){let r=s[n];t.shapes.push(r.uuid)}else t.shapes.push(s.uuid);return t.options=Object.assign({},e),e.extrudePath!==void 0&&(t.options.extrudePath=e.extrudePath.toJSON()),t}var Xi=class s extends Lt{constructor(e=1,t=1,n=1,i=1){super(),this.type="PlaneGeometry",this.parameters={width:e,height:t,widthSegments:n,heightSegments:i};let r=e/2,o=t/2,a=Math.floor(n),l=Math.floor(i),c=a+1,u=l+1,h=e/a,d=t/l,f=[],m=[],_=[],p=[];for(let g=0;g<u;g++){let S=g*d-o;for(let v=0;v<c;v++){let x=v*h-r;m.push(x,-S,0),_.push(0,0,1),p.push(v/a),p.push(1-g/l)}}for(let g=0;g<l;g++)for(let S=0;S<a;S++){let v=S+c*g,x=S+c*(g+1),b=S+1+c*(g+1),w=S+1+c*g;f.push(v,x,w),f.push(x,b,w)}this.setIndex(f),this.setAttribute("position",new ct(m,3)),this.setAttribute("normal",new ct(_,3)),this.setAttribute("uv",new ct(p,2))}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}static fromJSON(e){return new s(e.width,e.height,e.widthSegments,e.heightSegments)}};var yn=class s extends Lt{constructor(e=new kn([new pe(0,.5),new pe(-.5,-.5),new pe(.5,-.5)]),t=12){super(),this.type="ShapeGeometry",this.parameters={shapes:e,curveSegments:t};let n=[],i=[],r=[],o=[],a=0,l=0;if(Array.isArray(e)===!1)c(e);else for(let u=0;u<e.length;u++)c(e[u]),this.addGroup(a,l,u),a+=l,l=0;this.setIndex(n),this.setAttribute("position",new ct(i,3)),this.setAttribute("normal",new ct(r,3)),this.setAttribute("uv",new ct(o,2));function c(u){let h=i.length/3,d=u.extractPoints(t),f=d.shape,m=d.holes;cn.isClockWise(f)===!1&&(f=f.reverse());for(let p=0,g=m.length;p<g;p++){let S=m[p];cn.isClockWise(S)===!0&&(m[p]=S.reverse())}let _=cn.triangulateShape(f,m);for(let p=0,g=m.length;p<g;p++){let S=m[p];f=f.concat(S)}for(let p=0,g=f.length;p<g;p++){let S=f[p];i.push(S.x,S.y,0),r.push(0,0,1),o.push(S.x,S.y)}for(let p=0,g=_.length;p<g;p++){let S=_[p],v=S[0]+h,x=S[1]+h,b=S[2]+h;n.push(v,x,b),l+=3}}}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}toJSON(){let e=super.toJSON(),t=this.parameters.shapes;return fm(t,e)}static fromJSON(e,t){let n=[];for(let i=0,r=e.shapes.length;i<r;i++){let o=t[e.shapes[i]];n.push(o)}return new s(n,e.curveSegments)}};function fm(s,e){if(e.shapes=[],Array.isArray(s))for(let t=0,n=s.length;t<n;t++){let i=s[t];e.shapes.push(i.uuid)}else e.shapes.push(s.uuid);return e}var Ur=class s extends Lt{constructor(e=1,t=32,n=16,i=0,r=Math.PI*2,o=0,a=Math.PI){super(),this.type="SphereGeometry",this.parameters={radius:e,widthSegments:t,heightSegments:n,phiStart:i,phiLength:r,thetaStart:o,thetaLength:a},t=Math.max(3,Math.floor(t)),n=Math.max(2,Math.floor(n));let l=Math.min(o+a,Math.PI),c=0,u=[],h=new L,d=new L,f=[],m=[],_=[],p=[];for(let g=0;g<=n;g++){let S=[],v=g/n,x=o+v*a,b=e*Math.cos(x),w=Math.sqrt(e*e-b*b),y=0;g===0&&o===0?y=.5/t:g===n&&l===Math.PI&&(y=-.5/t);for(let M=0;M<=t;M++){let C=M/t,E=i+C*r;h.x=-w*Math.cos(E),h.y=b,h.z=w*Math.sin(E),m.push(h.x,h.y,h.z),d.copy(h).normalize(),_.push(d.x,d.y,d.z),p.push(C+y,1-v),S.push(c++)}u.push(S)}for(let g=0;g<n;g++)for(let S=0;S<t;S++){let v=u[g][S+1],x=u[g][S],b=u[g+1][S],w=u[g+1][S+1];(g!==0||o>0)&&f.push(v,x,w),(g!==n-1||l<Math.PI)&&f.push(x,b,w)}this.setIndex(f),this.setAttribute("position",new ct(m,3)),this.setAttribute("normal",new ct(_,3)),this.setAttribute("uv",new ct(p,2))}copy(e){return super.copy(e),this.parameters=Object.assign({},e.parameters),this}static fromJSON(e){return new s(e.radius,e.widthSegments,e.heightSegments,e.phiStart,e.phiLength,e.thetaStart,e.thetaLength)}};function es(s){let e={};for(let t in s){e[t]={};for(let n in s[t]){let i=s[t][n];if(Pu(i))i.isRenderTargetTexture?(Xe("UniformsUtils: Textures of render targets cannot be cloned via cloneUniforms() or mergeUniforms()."),e[t][n]=null):e[t][n]=i.clone();else if(Array.isArray(i))if(Pu(i[0])){let r=[];for(let o=0,a=i.length;o<a;o++)r[o]=i[o].clone();e[t][n]=r}else e[t][n]=i.slice();else e[t][n]=i}}return e}function jt(s){let e={};for(let t=0;t<s.length;t++){let n=es(s[t]);for(let i in n)e[i]=n[i]}return e}function Pu(s){return s&&(s.isColor||s.isMatrix3||s.isMatrix4||s.isVector2||s.isVector3||s.isVector4||s.isTexture||s.isQuaternion)}function dm(s){let e=[];for(let t=0;t<s.length;t++)e.push(s[t].clone());return e}function $c(s){let e=s.getRenderTarget();return e===null?s.outputColorSpace:e.isXRRenderTarget===!0?e.texture.colorSpace:lt.workingColorSpace}var Lf={clone:es,merge:jt},pm=`void main() {
	gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );
}`,mm=`void main() {
	gl_FragColor = vec4( 1.0, 0.0, 0.0, 1.0 );
}`,Vn=class extends ri{constructor(e){super(),this.isShaderMaterial=!0,this.type="ShaderMaterial",this.defines={},this.uniforms={},this.uniformsGroups=[],this.vertexShader=pm,this.fragmentShader=mm,this.linewidth=1,this.wireframe=!1,this.wireframeLinewidth=1,this.fog=!1,this.lights=!1,this.clipping=!1,this.forceSinglePass=!0,this.extensions={clipCullDistance:!1,multiDraw:!1},this.defaultAttributeValues={color:[1,1,1],uv:[0,0],uv1:[0,0]},this.index0AttributeName=void 0,this.uniformsNeedUpdate=!1,this.glslVersion=null,e!==void 0&&this.setValues(e)}copy(e){return super.copy(e),this.fragmentShader=e.fragmentShader,this.vertexShader=e.vertexShader,this.uniforms=es(e.uniforms),this.uniformsGroups=dm(e.uniformsGroups),this.defines=Object.assign({},e.defines),this.wireframe=e.wireframe,this.wireframeLinewidth=e.wireframeLinewidth,this.fog=e.fog,this.lights=e.lights,this.clipping=e.clipping,this.extensions=Object.assign({},e.extensions),this.glslVersion=e.glslVersion,this.defaultAttributeValues=Object.assign({},e.defaultAttributeValues),this.index0AttributeName=e.index0AttributeName,this.uniformsNeedUpdate=e.uniformsNeedUpdate,this}toJSON(e){let t=super.toJSON(e);t.glslVersion=this.glslVersion,t.uniforms={};for(let i in this.uniforms){let o=this.uniforms[i].value;o&&o.isTexture?t.uniforms[i]={type:"t",value:o.toJSON(e).uuid}:o&&o.isColor?t.uniforms[i]={type:"c",value:o.getHex()}:o&&o.isVector2?t.uniforms[i]={type:"v2",value:o.toArray()}:o&&o.isVector3?t.uniforms[i]={type:"v3",value:o.toArray()}:o&&o.isVector4?t.uniforms[i]={type:"v4",value:o.toArray()}:o&&o.isMatrix3?t.uniforms[i]={type:"m3",value:o.toArray()}:o&&o.isMatrix4?t.uniforms[i]={type:"m4",value:o.toArray()}:t.uniforms[i]={value:o}}Object.keys(this.defines).length>0&&(t.defines=this.defines),t.vertexShader=this.vertexShader,t.fragmentShader=this.fragmentShader,t.lights=this.lights,t.clipping=this.clipping;let n={};for(let i in this.extensions)this.extensions[i]===!0&&(n[i]=!0);return Object.keys(n).length>0&&(t.extensions=n),t}fromJSON(e,t){if(super.fromJSON(e,t),e.uniforms!==void 0)for(let n in e.uniforms){let i=e.uniforms[n];switch(this.uniforms[n]={},i.type){case"t":this.uniforms[n].value=t[i.value]||null;break;case"c":this.uniforms[n].value=new Ke().setHex(i.value);break;case"v2":this.uniforms[n].value=new pe().fromArray(i.value);break;case"v3":this.uniforms[n].value=new L().fromArray(i.value);break;case"v4":this.uniforms[n].value=new _t().fromArray(i.value);break;case"m3":this.uniforms[n].value=new Ze().fromArray(i.value);break;case"m4":this.uniforms[n].value=new ze().fromArray(i.value);break;default:this.uniforms[n].value=i.value}}if(e.defines!==void 0&&(this.defines=e.defines),e.vertexShader!==void 0&&(this.vertexShader=e.vertexShader),e.fragmentShader!==void 0&&(this.fragmentShader=e.fragmentShader),e.glslVersion!==void 0&&(this.glslVersion=e.glslVersion),e.extensions!==void 0)for(let n in e.extensions)this.extensions[n]=e.extensions[n];return e.lights!==void 0&&(this.lights=e.lights),e.clipping!==void 0&&(this.clipping=e.clipping),this}},ca=class extends Vn{constructor(e){super(e),this.isRawShaderMaterial=!0,this.type="RawShaderMaterial"}},fn=class extends ri{constructor(e){super(),this.isMeshStandardMaterial=!0,this.type="MeshStandardMaterial",this.defines={STANDARD:""},this.color=new Ke(16777215),this.roughness=1,this.metalness=0,this.map=null,this.lightMap=null,this.lightMapIntensity=1,this.aoMap=null,this.aoMapIntensity=1,this.emissive=new Ke(0),this.emissiveIntensity=1,this.emissiveMap=null,this.bumpMap=null,this.bumpScale=1,this.normalMap=null,this.normalMapType=$r,this.normalScale=new pe(1,1),this.displacementMap=null,this.displacementScale=1,this.displacementBias=0,this.roughnessMap=null,this.metalnessMap=null,this.alphaMap=null,this.envMap=null,this.envMapRotation=new Ut,this.envMapIntensity=1,this.wireframe=!1,this.wireframeLinewidth=1,this.wireframeLinecap="round",this.wireframeLinejoin="round",this.flatShading=!1,this.fog=!0,this.setValues(e)}copy(e){return super.copy(e),this.defines={STANDARD:""},this.color.copy(e.color),this.roughness=e.roughness,this.metalness=e.metalness,this.map=e.map,this.lightMap=e.lightMap,this.lightMapIntensity=e.lightMapIntensity,this.aoMap=e.aoMap,this.aoMapIntensity=e.aoMapIntensity,this.emissive.copy(e.emissive),this.emissiveMap=e.emissiveMap,this.emissiveIntensity=e.emissiveIntensity,this.bumpMap=e.bumpMap,this.bumpScale=e.bumpScale,this.normalMap=e.normalMap,this.normalMapType=e.normalMapType,this.normalScale.copy(e.normalScale),this.displacementMap=e.displacementMap,this.displacementScale=e.displacementScale,this.displacementBias=e.displacementBias,this.roughnessMap=e.roughnessMap,this.metalnessMap=e.metalnessMap,this.alphaMap=e.alphaMap,this.envMap=e.envMap,this.envMapRotation.copy(e.envMapRotation),this.envMapIntensity=e.envMapIntensity,this.wireframe=e.wireframe,this.wireframeLinewidth=e.wireframeLinewidth,this.wireframeLinecap=e.wireframeLinecap,this.wireframeLinejoin=e.wireframeLinejoin,this.flatShading=e.flatShading,this.fog=e.fog,this}},nn=class extends fn{constructor(e){super(),this.isMeshPhysicalMaterial=!0,this.defines={STANDARD:"",PHYSICAL:""},this.type="MeshPhysicalMaterial",this.anisotropyRotation=0,this.anisotropyMap=null,this.clearcoatMap=null,this.clearcoatRoughness=0,this.clearcoatRoughnessMap=null,this.clearcoatNormalScale=new pe(1,1),this.clearcoatNormalMap=null,this.diffuseRoughnessMap=null,this.ior=1.5,Object.defineProperty(this,"reflectivity",{get:function(){return it(2.5*(this.ior-1)/(this.ior+1),0,1)},set:function(t){this.ior=(1+.4*t)/(1-.4*t)}}),this.iridescenceMap=null,this.iridescenceIOR=1.3,this.iridescenceThicknessMinimum=100,this.iridescenceThicknessMaximum=400,this.iridescenceThicknessMap=null,this.sheenColor=new Ke(0),this.sheenColorMap=null,this.sheenRoughness=1,this.sheenRoughnessMap=null,this.transmissionMap=null,this.thickness=0,this.thicknessMap=null,this.attenuationDistance=1/0,this.attenuationColor=new Ke(1,1,1),this.specularIntensity=1,this.specularIntensityMap=null,this.specularColor=new Ke(1,1,1),this.specularColorMap=null,this._anisotropy=0,this._clearcoat=0,this._diffuseRoughness=0,this._dispersion=0,this._iridescence=0,this._retroreflectivity=0,this._sheen=0,this._transmission=0,this.setValues(e)}get anisotropy(){return this._anisotropy}set anisotropy(e){this._anisotropy>0!=e>0&&this.version++,this._anisotropy=e}get clearcoat(){return this._clearcoat}set clearcoat(e){this._clearcoat>0!=e>0&&this.version++,this._clearcoat=e}get diffuseRoughness(){return this._diffuseRoughness}set diffuseRoughness(e){this._diffuseRoughness>0!=e>0&&this.version++,this._diffuseRoughness=e}get iridescence(){return this._iridescence}set iridescence(e){this._iridescence>0!=e>0&&this.version++,this._iridescence=e}get dispersion(){return this._dispersion}set dispersion(e){this._dispersion>0!=e>0&&this.version++,this._dispersion=e}get retroreflectivity(){return this._retroreflectivity}set retroreflectivity(e){this._retroreflectivity>0!=e>0&&this.version++,this._retroreflectivity=e}get sheen(){return this._sheen}set sheen(e){this._sheen>0!=e>0&&this.version++,this._sheen=e}get transmission(){return this._transmission}set transmission(e){this._transmission>0!=e>0&&this.version++,this._transmission=e}get iridescenceThicknessRange(){return Vi('MeshPhysicalMaterial: The "iridescenceThicknessRange" property has been replaced by "iridescenceThicknessMinimum" and "iridescenceThicknessMaximum".'),[this.iridescenceThicknessMinimum,this.iridescenceThicknessMaximum]}set iridescenceThicknessRange(e){Vi('MeshPhysicalMaterial: The "iridescenceThicknessRange" property has been replaced by "iridescenceThicknessMinimum" and "iridescenceThicknessMaximum".'),this.iridescenceThicknessMinimum=e[0],this.iridescenceThicknessMaximum=e[1]}copy(e){return super.copy(e),this.defines={STANDARD:"",PHYSICAL:""},this.anisotropy=e.anisotropy,this.anisotropyRotation=e.anisotropyRotation,this.anisotropyMap=e.anisotropyMap,this.clearcoat=e.clearcoat,this.clearcoatMap=e.clearcoatMap,this.clearcoatRoughness=e.clearcoatRoughness,this.clearcoatRoughnessMap=e.clearcoatRoughnessMap,this.clearcoatNormalMap=e.clearcoatNormalMap,this.clearcoatNormalScale.copy(e.clearcoatNormalScale),this.diffuseRoughness=e.diffuseRoughness,this.diffuseRoughnessMap=e.diffuseRoughnessMap,this.dispersion=e.dispersion,this.ior=e.ior,this.iridescence=e.iridescence,this.iridescenceMap=e.iridescenceMap,this.iridescenceIOR=e.iridescenceIOR,this.iridescenceThicknessMinimum=e.iridescenceThicknessMinimum,this.iridescenceThicknessMaximum=e.iridescenceThicknessMaximum,this.iridescenceThicknessMap=e.iridescenceThicknessMap,this.retroreflectivity=e.retroreflectivity,this.sheen=e.sheen,this.sheenColor.copy(e.sheenColor),this.sheenColorMap=e.sheenColorMap,this.sheenRoughness=e.sheenRoughness,this.sheenRoughnessMap=e.sheenRoughnessMap,this.transmission=e.transmission,this.transmissionMap=e.transmissionMap,this.thickness=e.thickness,this.thicknessMap=e.thicknessMap,this.attenuationDistance=e.attenuationDistance,this.attenuationColor.copy(e.attenuationColor),this.specularIntensity=e.specularIntensity,this.specularIntensityMap=e.specularIntensityMap,this.specularColor.copy(e.specularColor),this.specularColorMap=e.specularColorMap,this}};var Fr=class extends ri{constructor(e){super(),this.isMeshLambertMaterial=!0,this.type="MeshLambertMaterial",this.color=new Ke(16777215),this.map=null,this.lightMap=null,this.lightMapIntensity=1,this.aoMap=null,this.aoMapIntensity=1,this.emissive=new Ke(0),this.emissiveIntensity=1,this.emissiveMap=null,this.bumpMap=null,this.bumpScale=1,this.normalMap=null,this.normalMapType=$r,this.normalScale=new pe(1,1),this.displacementMap=null,this.displacementScale=1,this.displacementBias=0,this.specularMap=null,this.alphaMap=null,this.envMap=null,this.envMapRotation=new Ut,this.combine=va,this.reflectivity=1,this.envMapIntensity=1,this.refractionRatio=.98,this.wireframe=!1,this.wireframeLinewidth=1,this.wireframeLinecap="round",this.wireframeLinejoin="round",this.flatShading=!1,this.fog=!0,this.setValues(e)}copy(e){return super.copy(e),this.color.copy(e.color),this.map=e.map,this.lightMap=e.lightMap,this.lightMapIntensity=e.lightMapIntensity,this.aoMap=e.aoMap,this.aoMapIntensity=e.aoMapIntensity,this.emissive.copy(e.emissive),this.emissiveMap=e.emissiveMap,this.emissiveIntensity=e.emissiveIntensity,this.bumpMap=e.bumpMap,this.bumpScale=e.bumpScale,this.normalMap=e.normalMap,this.normalMapType=e.normalMapType,this.normalScale.copy(e.normalScale),this.displacementMap=e.displacementMap,this.displacementScale=e.displacementScale,this.displacementBias=e.displacementBias,this.specularMap=e.specularMap,this.alphaMap=e.alphaMap,this.envMap=e.envMap,this.envMapRotation.copy(e.envMapRotation),this.combine=e.combine,this.reflectivity=e.reflectivity,this.envMapIntensity=e.envMapIntensity,this.refractionRatio=e.refractionRatio,this.wireframe=e.wireframe,this.wireframeLinewidth=e.wireframeLinewidth,this.wireframeLinecap=e.wireframeLinecap,this.wireframeLinejoin=e.wireframeLinejoin,this.flatShading=e.flatShading,this.fog=e.fog,this}},ha=class extends ri{constructor(e){super(),this.isMeshDepthMaterial=!0,this.type="MeshDepthMaterial",this.depthPacking=pf,this.map=null,this.alphaMap=null,this.displacementMap=null,this.displacementScale=1,this.displacementBias=0,this.wireframe=!1,this.wireframeLinewidth=1,this.setValues(e)}copy(e){return super.copy(e),this.depthPacking=e.depthPacking,this.map=e.map,this.alphaMap=e.alphaMap,this.displacementMap=e.displacementMap,this.displacementScale=e.displacementScale,this.displacementBias=e.displacementBias,this.wireframe=e.wireframe,this.wireframeLinewidth=e.wireframeLinewidth,this}},ua=class extends ri{constructor(e){super(),this.isMeshDistanceMaterial=!0,this.type="MeshDistanceMaterial",this.map=null,this.alphaMap=null,this.displacementMap=null,this.displacementScale=1,this.displacementBias=0,this.setValues(e)}copy(e){return super.copy(e),this.map=e.map,this.alphaMap=e.alphaMap,this.displacementMap=e.displacementMap,this.displacementScale=e.displacementScale,this.displacementBias=e.displacementBias,this}};function bi(s,e){return!s||s.constructor===e?s:typeof e.BYTES_PER_ELEMENT=="number"?new e(s):Array.prototype.slice.call(s)}function hr(s){return s!==void 0&&s.inTangents!==void 0&&s.outTangents!==void 0}function gm(s){function e(i,r){return s[i]-s[r]}let t=s.length,n=new Array(t);for(let i=0;i!==t;++i)n[i]=i;return n.sort(e),n}function Iu(s,e,t){let n=s.length,i=new s.constructor(n);for(let r=0,o=0;o!==n;++r){let a=t[r]*e;for(let l=0;l!==e;++l)i[o++]=s[a+l]}return i}function _m(s,e,t,n){let i=1,r=s[0];for(;r!==void 0&&r[n]===void 0;)r=s[i++];if(r===void 0)return;let o=r[n];if(o!==void 0)if(Array.isArray(o))do o=r[n],o!==void 0&&(e.push(r.time),t.push(...o)),r=s[i++];while(r!==void 0);else if(o.toArray!==void 0)do o=r[n],o!==void 0&&(e.push(r.time),o.toArray(t,t.length)),r=s[i++];while(r!==void 0);else do o=r[n],o!==void 0&&(e.push(r.time),t.push(o)),r=s[i++];while(r!==void 0)}var Ai=class{constructor(e,t,n,i){this.parameterPositions=e,this._cachedIndex=0,this.resultBuffer=i!==void 0?i:new t.constructor(n),this.sampleValues=t,this.valueSize=n,this.settings=null,this.DefaultSettings_={}}evaluate(e){let t=this.parameterPositions,n=this._cachedIndex,i=t[n],r=t[n-1];n:{e:{let o;t:{i:if(!(e<i)){for(let a=n+2;;){if(i===void 0){if(e<r)break i;return n=t.length,this._cachedIndex=n,this.copySampleValue_(n-1)}if(n===a)break;if(r=i,i=t[++n],e<i)break e}o=t.length;break t}if(!(e>=r)){let a=t[1];e<a&&(n=2,r=a);for(let l=n-2;;){if(r===void 0)return this._cachedIndex=0,this.copySampleValue_(0);if(n===l)break;if(i=r,r=t[--n-1],e>=r)break e}o=n,n=0;break t}break n}for(;n<o;){let a=n+o>>>1;e<t[a]?o=a:n=a+1}if(i=t[n],r=t[n-1],r===void 0)return this._cachedIndex=0,this.copySampleValue_(0);if(i===void 0)return n=t.length,this._cachedIndex=n,this.copySampleValue_(n-1)}this._cachedIndex=n,this.intervalChanged_(n,r,i)}return this.interpolate_(n,r,e,i)}getSettings_(){return this.settings||this.DefaultSettings_}copySampleValue_(e){let t=this.resultBuffer,n=this.sampleValues,i=this.valueSize,r=e*i;for(let o=0;o!==i;++o)t[o]=n[r+o];return t}interpolate_(){throw new Error("THREE.Interpolant: Call to abstract method.")}intervalChanged_(){}},fa=class extends Ai{constructor(e,t,n,i){super(e,t,n,i),this._weightPrev=-0,this._offsetPrev=-0,this._weightNext=-0,this._offsetNext=-0,this.DefaultSettings_={endingStart:cc,endingEnd:cc}}intervalChanged_(e,t,n){let i=this.parameterPositions,r=e-2,o=e+1,a=i[r],l=i[o];if(a===void 0)switch(this.getSettings_().endingStart){case hc:r=e,a=2*t-n;break;case uc:r=i.length-2,a=t+i[r]-i[r+1];break;default:r=e,a=n}if(l===void 0)switch(this.getSettings_().endingEnd){case hc:o=e,l=2*n-t;break;case uc:o=1,l=n+i[1]-i[0];break;default:o=e-1,l=t}let c=(n-t)*.5,u=this.valueSize;this._weightPrev=c/(t-a),this._weightNext=c/(l-n),this._offsetPrev=r*u,this._offsetNext=o*u}interpolate_(e,t,n,i){let r=this.resultBuffer,o=this.sampleValues,a=this.valueSize,l=e*a,c=l-a,u=this._offsetPrev,h=this._offsetNext,d=this._weightPrev,f=this._weightNext,m=(n-t)/(i-t),_=m*m,p=_*m,g=-d*p+2*d*_-d*m,S=(1+d)*p+(-1.5-2*d)*_+(-.5+d)*m+1,v=(-1-f)*p+(1.5+f)*_+.5*m,x=f*p-f*_;for(let b=0;b!==a;++b)r[b]=g*o[u+b]+S*o[c+b]+v*o[l+b]+x*o[h+b];return r}},da=class extends Ai{constructor(e,t,n,i){super(e,t,n,i)}interpolate_(e,t,n,i){let r=this.resultBuffer,o=this.sampleValues,a=this.valueSize,l=e*a,c=l-a,u=(n-t)/(i-t),h=1-u;for(let d=0;d!==a;++d)r[d]=o[c+d]*h+o[l+d]*u;return r}},pa=class extends Ai{constructor(e,t,n,i){super(e,t,n,i)}interpolate_(e){return this.copySampleValue_(e-1)}},ma=class extends Ai{interpolate_(e,t,n,i){let r=this.resultBuffer,o=this.sampleValues,a=this.valueSize,l=e*a,c=l-a,u=this.inTangents,h=this.outTangents;if(!u||!h){let m=(n-t)/(i-t),_=1-m;for(let p=0;p!==a;++p)r[p]=o[c+p]*_+o[l+p]*m;return r}let d=a*2,f=e-1;for(let m=0;m!==a;++m){let _=o[c+m],p=o[l+m],g=f*d+m*2,S=h[g],v=h[g+1],x=e*d+m*2,b=u[x],w=u[x+1],y=ym(n,t,S,b,i);r[m]=Nf(y,_,v,w,p)}return r}};function Nf(s,e,t,n,i){let r=1-s;return r*r*r*e+3*r*r*s*t+3*r*s*s*n+s*s*s*i}function xm(s,e,t,n,i){let r=1-s;return 3*r*r*(t-e)+6*r*s*(n-t)+3*s*s*(i-n)}function ym(s,e,t,n,i){let r=(s-e)/(i-e);for(let o=0;o<8;o++){let a=Nf(r,e,t,n,i)-s;if(Math.abs(a)<1e-10)break;let l=xm(r,e,t,n,i);if(Math.abs(l)<1e-10)break;r=Math.max(0,Math.min(1,r-a/l))}return r}var rn=class{constructor(e,t,n,i){if(e===void 0)throw new Error("THREE.KeyframeTrack: track name is undefined");if(t===void 0||t.length===0)throw new Error("THREE.KeyframeTrack: no keyframes in track named "+e);this.name=e,this.times=bi(t,this.TimeBufferType),this.values=bi(n,this.ValueBufferType),this.setInterpolation(i||this.DefaultInterpolation)}static toJSON(e){let t=e.constructor,n;if(t.toJSON!==this.toJSON)n=t.toJSON(e);else{n={name:e.name,times:bi(e.times,Array),values:bi(e.values,Array)};let i=e.getInterpolation();i!==e.DefaultInterpolation&&(n.interpolation=i),hr(e.settings)&&(n.settings={inTangents:bi(e.settings.inTangents,Array),outTangents:bi(e.settings.outTangents,Array)})}return n.type=e.ValueTypeName,n}InterpolantFactoryMethodDiscrete(e){return new pa(this.times,this.values,this.getValueSize(),e)}InterpolantFactoryMethodLinear(e){return new da(this.times,this.values,this.getValueSize(),e)}InterpolantFactoryMethodSmooth(e){return new fa(this.times,this.values,this.getValueSize(),e)}InterpolantFactoryMethodBezier(e){let t=new ma(this.times,this.values,this.getValueSize(),e);return this.settings&&(t.inTangents=this.settings.inTangents,t.outTangents=this.settings.outTangents),t}setInterpolation(e){let t;switch(e){case dr:t=this.InterpolantFactoryMethodDiscrete;break;case jo:t=this.InterpolantFactoryMethodLinear;break;case Ho:t=this.InterpolantFactoryMethodSmooth;break;case lc:t=this.InterpolantFactoryMethodBezier;break}if(t===void 0){let n="unsupported interpolation for "+this.ValueTypeName+" keyframe track named "+this.name;if(this.createInterpolant===void 0)if(e!==this.DefaultInterpolation)this.setInterpolation(this.DefaultInterpolation);else throw new Error(n);return Xe("KeyframeTrack:",n),this}return this.createInterpolant=t,this}getInterpolation(){switch(this.createInterpolant){case this.InterpolantFactoryMethodDiscrete:return dr;case this.InterpolantFactoryMethodLinear:return jo;case this.InterpolantFactoryMethodSmooth:return Ho;case this.InterpolantFactoryMethodBezier:return lc}}getValueSize(){return this.values.length/this.times.length}shift(e){if(e!==0){let t=this.times;for(let n=0,i=t.length;n!==i;++n)t[n]+=e;hr(this.settings)&&(Du(this.settings.inTangents,e),Du(this.settings.outTangents,e))}return this}scale(e){if(e!==1){let t=this.times;for(let n=0,i=t.length;n!==i;++n)t[n]*=e;hr(this.settings)&&(Lu(this.settings.inTangents,e),Lu(this.settings.outTangents,e))}return this}trim(e,t){let n=this.times,i=n.length,r=0,o=i-1;for(;r!==i&&n[r]<e;)++r;for(;o!==-1&&n[o]>t;)--o;if(++o,r!==0||o!==i){r>=o&&(o=Math.max(o,1),r=o-1);let a=this.getValueSize();this.times=n.slice(r,o),this.values=this.values.slice(r*a,o*a)}return this}validate(){let e=!0,t=this.getValueSize();t-Math.floor(t)!==0&&(Ye("KeyframeTrack: Invalid value size in track.",this),e=!1);let n=this.times,i=this.values,r=n.length;r===0&&(Ye("KeyframeTrack: Track is empty.",this),e=!1);let o=null;for(let a=0;a!==r;a++){let l=n[a];if(typeof l=="number"&&isNaN(l)){Ye("KeyframeTrack: Time is not a valid number.",this,a,l),e=!1;break}if(o!==null&&o>l){Ye("KeyframeTrack: Out of order keys.",this,a,l,o),e=!1;break}o=l}if(i!==void 0&&mp(i))for(let a=0,l=i.length;a!==l;++a){let c=i[a];if(isNaN(c)){Ye("KeyframeTrack: Value is not a valid number.",this,a,c),e=!1;break}}return e}optimize(){let e=this.times.slice(),t=this.values.slice(),n=this.getValueSize(),i=this.getInterpolation()===Ho,r=e.length-1,o=1;for(let a=1;a<r;++a){let l=!1,c=e[a],u=e[a+1];if(c!==u&&(a!==1||c!==e[0]))if(i)l=!0;else{let h=a*n,d=h-n,f=h+n;for(let m=0;m!==n;++m){let _=t[h+m];if(_!==t[d+m]||_!==t[f+m]){l=!0;break}}}if(l){if(a!==o){e[o]=e[a];let h=a*n,d=o*n;for(let f=0;f!==n;++f)t[d+f]=t[h+f]}++o}}if(r>0){e[o]=e[r];for(let a=r*n,l=o*n,c=0;c!==n;++c)t[l+c]=t[a+c];++o}return o!==e.length?(this.times=e.slice(0,o),this.values=t.slice(0,o*n)):(this.times=e,this.values=t),this}clone(){let e=this.times.slice(),t=this.values.slice(),n=this.constructor,i=new n(this.name,e,t);return i.createInterpolant=this.createInterpolant,hr(this.settings)&&(i.settings={inTangents:this.settings.inTangents.slice(),outTangents:this.settings.outTangents.slice()}),i}};function Du(s,e){for(let t=0,n=s.length;t!==n;t+=2)s[t]+=e}function Lu(s,e){for(let t=0,n=s.length;t!==n;t+=2)s[t]*=e}rn.prototype.ValueTypeName="";rn.prototype.TimeBufferType=Float32Array;rn.prototype.ValueBufferType=Float32Array;rn.prototype.DefaultInterpolation=jo;var ci=class extends rn{constructor(e,t,n){super(e,t,n)}};ci.prototype.ValueTypeName="bool";ci.prototype.ValueBufferType=Array;ci.prototype.DefaultInterpolation=dr;ci.prototype.InterpolantFactoryMethodLinear=void 0;ci.prototype.InterpolantFactoryMethodSmooth=void 0;var Or=class extends rn{constructor(e,t,n,i){super(e,t,n,i)}};Or.prototype.ValueTypeName="color";var Os=class extends rn{constructor(e,t,n,i){super(e,t,n,i)}};Os.prototype.ValueTypeName="number";var ga=class extends Ai{constructor(e,t,n,i){super(e,t,n,i)}interpolate_(e,t,n,i){let r=this.resultBuffer,o=this.sampleValues,a=this.valueSize,l=(n-t)/(i-t),c=e*a;for(let u=c+a;c!==u;c+=4)st.slerpFlat(r,0,o,c-a,o,c,l);return r}},Cn=class extends rn{constructor(e,t,n,i){super(e,t,n,i)}InterpolantFactoryMethodLinear(e){return new ga(this.times,this.values,this.getValueSize(),e)}};Cn.prototype.ValueTypeName="quaternion";Cn.prototype.InterpolantFactoryMethodSmooth=void 0;var hi=class extends rn{constructor(e,t,n){super(e,t,n)}};hi.prototype.ValueTypeName="string";hi.prototype.ValueBufferType=Array;hi.prototype.DefaultInterpolation=dr;hi.prototype.InterpolantFactoryMethodLinear=void 0;hi.prototype.InterpolantFactoryMethodSmooth=void 0;var vn=class extends rn{constructor(e,t,n,i){super(e,t,n,i)}};vn.prototype.ValueTypeName="vector";var Bs=class{constructor(e="",t=-1,n=[],i=df){this.name=e,this.tracks=n,this.duration=t,this.blendMode=i,this.uuid=Pi(),this.userData={},this.duration<0&&this.resetDuration()}static parse(e){let t=[],n=e.tracks,i=1/(e.fps||1);for(let o=0,a=n.length;o!==a;++o)t.push(Sm(n[o]).scale(i));let r=new this(e.name,e.duration,t,e.blendMode);return r.uuid=e.uuid,r.userData=JSON.parse(e.userData||"{}"),r}static toJSON(e){let t=[],n=e.tracks,i={name:e.name,duration:e.duration,tracks:t,uuid:e.uuid,blendMode:e.blendMode,userData:JSON.stringify(e.userData)};for(let r=0,o=n.length;r!==o;++r)t.push(rn.toJSON(n[r]));return i}static CreateFromMorphTargetSequence(e,t,n,i){let r=t.length,o=[];for(let a=0;a<r;a++){let l=[],c=[];l.push((a+r-1)%r,a,(a+1)%r),c.push(0,1,0);let u=gm(l);l=Iu(l,1,u),c=Iu(c,1,u),!i&&l[0]===0&&(l.push(r),c.push(c[0])),o.push(new Os(".morphTargetInfluences["+t[a].name+"]",l,c).scale(1/n))}return new this(e,-1,o)}static findByName(e,t){let n=e;if(!Array.isArray(e)){let i=e;n=i.geometry&&i.geometry.animations||i.animations}for(let i=0;i<n.length;i++)if(n[i].name===t)return n[i];return null}static CreateClipsFromMorphTargetSequences(e,t,n){let i={},r=/^([\w-]*?)([\d]+)$/;for(let a=0,l=e.length;a<l;a++){let c=e[a],u=c.name.match(r);if(u&&u.length>1){let h=u[1],d=i[h];d||(i[h]=d=[]),d.push(c)}}let o=[];for(let a in i)o.push(this.CreateFromMorphTargetSequence(a,i[a],t,n));return o}resetDuration(){let e=this.tracks,t=0;for(let n=0,i=e.length;n!==i;++n){let r=this.tracks[n];t=Math.max(t,r.times[r.times.length-1])}return this.duration=t,this}trim(){for(let e=0;e<this.tracks.length;e++)this.tracks[e].trim(0,this.duration);return this}validate(){let e=!0;for(let t=0;t<this.tracks.length;t++)e=e&&this.tracks[t].validate();return e}optimize(){for(let e=0;e<this.tracks.length;e++)this.tracks[e].optimize();return this}clone(){let e=[];for(let n=0;n<this.tracks.length;n++)e.push(this.tracks[n].clone());let t=new this.constructor(this.name,this.duration,e,this.blendMode);return t.userData=JSON.parse(JSON.stringify(this.userData)),t}toJSON(){return this.constructor.toJSON(this)}};function vm(s){switch(s.toLowerCase()){case"scalar":case"double":case"float":case"number":case"integer":return Os;case"vector":case"vector2":case"vector3":case"vector4":return vn;case"color":return Or;case"quaternion":return Cn;case"bool":case"boolean":return ci;case"string":return hi}throw new Error("THREE.KeyframeTrack: Unsupported typeName: "+s)}function Sm(s){if(s.type===void 0)throw new Error("THREE.KeyframeTrack: track type undefined, can not parse");let e=vm(s.type);if(s.times===void 0){let n=[],i=[];_m(s.keys,n,i,"value"),s.times=n,s.values=i}let t;return e.parse!==void 0?t=e.parse(s):t=new e(s.name,s.times,s.values,s.interpolation),hr(s.settings)&&(t.settings={inTangents:bi(s.settings.inTangents,Float32Array),outTangents:bi(s.settings.outTangents,Float32Array)}),t}var Ms={enabled:!1,files:{},add:function(s,e){this.enabled!==!1&&(Nu(s)||(this.files[s]=e))},get:function(s){if(this.enabled!==!1&&!Nu(s))return this.files[s]},remove:function(s){delete this.files[s]},clear:function(){this.files={}}};function Nu(s){try{let e=s.slice(s.indexOf(":")+1);return new URL(e).protocol==="blob:"}catch{return!1}}var _a=class{constructor(e,t,n){let i=this,r=!1,o=0,a=0,l,c=[];this.onStart=void 0,this.onLoad=e,this.onProgress=t,this.onError=n,this._abortController=null,this.itemStart=function(u){a++,r===!1&&i.onStart!==void 0&&i.onStart(u,o,a),r=!0},this.itemEnd=function(u){o++,i.onProgress!==void 0&&i.onProgress(u,o,a),o===a&&(r=!1,i.onLoad!==void 0&&i.onLoad())},this.itemError=function(u){i.onError!==void 0&&i.onError(u)},this.resolveURL=function(u){return u=u.normalize("NFC"),l?l(u):u},this.setURLModifier=function(u){return l=u,this},this.addHandler=function(u,h){return c.push(u,h),this},this.removeHandler=function(u){let h=c.indexOf(u);return h!==-1&&c.splice(h,2),this},this.getHandler=function(u){for(let h=0,d=c.length;h<d;h+=2){let f=c[h],m=c[h+1];if(f.global&&(f.lastIndex=0),f.test(u))return m}return null},this.abort=function(){return this.abortController.abort(),this._abortController=null,this}}get abortController(){return this._abortController||(this._abortController=new AbortController),this._abortController}},Uf=new _a,Ti=class{constructor(e){this.manager=e!==void 0?e:Uf,this.crossOrigin="anonymous",this.withCredentials=!1,this.path="",this.resourcePath="",this.requestHeader={},this.priority="auto",this.cache="default",typeof __THREE_DEVTOOLS__<"u"&&__THREE_DEVTOOLS__.dispatchEvent(new CustomEvent("observe",{detail:this}))}load(){}loadAsync(e,t){let n=this;return new Promise(function(i,r){n.load(e,i,t,r)})}parse(){}setCrossOrigin(e){return this.crossOrigin=e,this}setWithCredentials(e){return this.withCredentials=e,this}setPath(e){return this.path=e,this}setResourcePath(e){return this.resourcePath=e,this}setRequestHeader(e){return this.requestHeader=e,this}setPriority(e){return this.priority=e,this}setCache(e){return this.cache=e,this}abort(){return this}};Ti.DEFAULT_MATERIAL_NAME="__DEFAULT";var ti={},gc=class extends Error{constructor(e,t){super(e),this.response=t}},Br=class extends Ti{constructor(e){super(e),this.mimeType="",this.responseType="",this._abortController=new AbortController}load(e,t,n,i){e===void 0&&(e=""),this.path!==void 0&&(e=this.path+e),e=this.manager.resolveURL(e);let r=Ms.get(`file:${e}`);if(r!==void 0){this.manager.itemStart(e),setTimeout(()=>{t&&t(r),this.manager.itemEnd(e)},0);return}if(ti[e]!==void 0){ti[e].push({onLoad:t,onProgress:n,onError:i});return}ti[e]=[],ti[e].push({onLoad:t,onProgress:n,onError:i});let o=new Request(e,{headers:new Headers(this.requestHeader),credentials:this.withCredentials?"include":"same-origin",priority:this.priority,cache:this.cache,signal:typeof AbortSignal.any=="function"?AbortSignal.any([this._abortController.signal,this.manager.abortController.signal]):this._abortController.signal}),a=this.mimeType,l=this.responseType;fetch(o).then(c=>{if(c.status===200||c.status===0){if(c.status===0&&Xe("FileLoader: HTTP Status 0 received."),typeof ReadableStream>"u"||c.body===void 0||c.body.getReader===void 0)return c;let u=ti[e],h=c.body.getReader(),d=c.headers.get("X-File-Size")||c.headers.get("Content-Length"),f=d?parseInt(d):0,m=f!==0,_=0,p=new ReadableStream({start(g){S();function S(){h.read().then(({done:v,value:x})=>{if(v)g.close();else{_+=x.byteLength;let b=new ProgressEvent("progress",{lengthComputable:m,loaded:_,total:f});for(let w=0,y=u.length;w<y;w++){let M=u[w];M.onProgress&&M.onProgress(b)}g.enqueue(x),S()}},v=>{g.error(v)})}}});return new Response(p)}else throw new gc(`fetch for "${c.url}" responded with ${c.status}: ${c.statusText}`,c)}).then(c=>{switch(l){case"arraybuffer":return c.arrayBuffer();case"blob":return c.blob();case"document":return c.text().then(u=>new DOMParser().parseFromString(u,a));case"json":return c.json();default:if(a==="")return c.text();{let h=/charset="?([^;"\s]*)"?/i.exec(a),d=h&&h[1]?h[1].toLowerCase():void 0,f=new TextDecoder(d);return c.arrayBuffer().then(m=>f.decode(m))}}}).then(c=>{Ms.add(`file:${e}`,c);let u=ti[e];delete ti[e];for(let h=0,d=u.length;h<d;h++){let f=u[h];f.onLoad&&f.onLoad(c)}}).catch(c=>{let u=ti[e];if(u===void 0)throw this.manager.itemError(e),c;delete ti[e];for(let h=0,d=u.length;h<d;h++){let f=u[h];f.onError&&f.onError(c)}this.manager.itemError(e)}).finally(()=>{this.manager.itemEnd(e)}),this.manager.itemStart(e)}setResponseType(e){return this.responseType=e,this}setMimeType(e){return this.mimeType=e,this}abort(){return this._abortController.abort(),this._abortController=new AbortController,this}};var xs=new WeakMap,zr=class extends Ti{constructor(e){super(e)}load(e,t,n,i){this.path!==void 0&&(e=this.path+e),e=this.manager.resolveURL(e);let r=this,o=Ms.get(`image:${e}`);if(o!==void 0){if(o.complete===!0)r.manager.itemStart(e),setTimeout(function(){t&&t(o),r.manager.itemEnd(e)},0);else{let h=xs.get(o);h===void 0&&(h=[],xs.set(o,h)),h.push({onLoad:t,onError:i})}return o}let a=Ts("img");function l(){u(),t&&t(this);let h=xs.get(this)||[];for(let d=0;d<h.length;d++){let f=h[d];f.onLoad&&f.onLoad(this)}xs.delete(this),r.manager.itemEnd(e)}function c(h){u(),i&&i(h),Ms.remove(`image:${e}`);let d=xs.get(this)||[];for(let f=0;f<d.length;f++){let m=d[f];m.onError&&m.onError(h)}xs.delete(this),r.manager.itemError(e),r.manager.itemEnd(e)}function u(){a.removeEventListener("load",l,!1),a.removeEventListener("error",c,!1)}return a.addEventListener("load",l,!1),a.addEventListener("error",c,!1),e.slice(0,5)!=="data:"&&this.crossOrigin!==void 0&&(a.crossOrigin=this.crossOrigin),Ms.add(`image:${e}`,a),r.manager.itemStart(e),a.fetchPriority=this.priority,a.src=e,a}};var Ei=class extends At{static registerNode(e){this.prototype._lightNode=e}constructor(e,t=1){super(),this.isLight=!0,this.type="Light",this.color=new Ke(e),this.intensity=t}copy(e,t){return super.copy(e,t),this.color.copy(e.color),this.intensity=e.intensity,this}toJSON(e){let t=super.toJSON(e);return t.object.color=this.color.getHex(),t.object.intensity=this.intensity,t}};var oc=new ze,Uu=new L,Fu=new L,zs=class{constructor(e){this.camera=e,this.intensity=1,this.bias=0,this.biasNode=null,this.normalBias=0,this.radius=1,this.blurSamples=8,this.mapSize=new pe(512,512),this.mapType=on,this.map=null,this.mapPass=null,this.matrix=new ze,this.autoUpdate=!0,this.needsUpdate=!1,this._frustum=new wi,this._frameExtents=new pe(1,1),this._viewportCount=1,this._viewports=[new _t(0,0,1,1)]}getViewportCount(){return this._viewportCount}getCamera(){return this.camera}getFrustum(){return this._frustum}updateMatrices(e){let t=this.camera;Uu.setFromMatrixPosition(e.matrixWorld),t.position.copy(Uu),Fu.setFromMatrixPosition(e.target.matrixWorld),t.lookAt(Fu),t.updateMatrixWorld(),this._updateMatrix(t,this.matrix,this._frustum)}_updateMatrix(e,t,n,i){oc.multiplyMatrices(e.projectionMatrix,e.matrixWorldInverse),n.setFromProjectionMatrix(oc,e.coordinateSystem,e.reversedDepth);let r=this._frameExtents,o=i?i.z/r.x:1,a=i?i.w/r.y:1,l=i?i.x/r.x:0,c=i?i.y/r.y:0;e.coordinateSystem===As||e.reversedDepth?t.set(.5*o,0,0,.5*o+l,0,.5*a,0,.5*a+c,0,0,1,0,0,0,0,1):t.set(.5*o,0,0,.5*o+l,0,.5*a,0,.5*a+c,0,0,.5,.5,0,0,0,1),t.multiply(oc)}getViewport(e){return this._viewports[e]}getFrameExtents(){return this._frameExtents}dispose(){this.map&&this.map.dispose(),this.mapPass&&this.mapPass.dispose()}copy(e){return this.camera=e.camera.clone(),this.intensity=e.intensity,this.bias=e.bias,this.radius=e.radius,this.autoUpdate=e.autoUpdate,this.needsUpdate=e.needsUpdate,this.normalBias=e.normalBias,this.blurSamples=e.blurSamples,this.mapSize.copy(e.mapSize),this.mapType=e.mapType,this.biasNode=e.biasNode,this}clone(){return new this.constructor().copy(this)}toJSON(){let e={};return e.intensity=this.intensity,e.bias=this.bias,e.normalBias=this.normalBias,e.radius=this.radius,e.blurSamples=this.blurSamples,e.mapSize=this.mapSize.toArray(),e.mapType=this.mapType,e.camera=this.camera.toJSON(!1).object,delete e.camera.matrix,e}},Vo=new L,Go=new st,Fn=new L,kr=class extends At{constructor(){super(),this.isCamera=!0,this.type="Camera",this.matrixWorldInverse=new ze,this.projectionMatrix=new ze,this.projectionMatrixInverse=new ze,this.coordinateSystem=xn,this._reversedDepth=!1}get reversedDepth(){return this._reversedDepth}copy(e,t){return super.copy(e,t),this.matrixWorldInverse.copy(e.matrixWorldInverse),this.projectionMatrix.copy(e.projectionMatrix),this.projectionMatrixInverse.copy(e.projectionMatrixInverse),this.coordinateSystem=e.coordinateSystem,this}getWorldDirection(e){return super.getWorldDirection(e).negate()}updateMatrixWorld(e){super.updateMatrixWorld(e),this.matrixWorld.decompose(Vo,Go,Fn),Fn.x===1&&Fn.y===1&&Fn.z===1?this.matrixWorldInverse.copy(this.matrixWorld).invert():this.matrixWorldInverse.compose(Vo,Go,Fn.set(1,1,1)).invert()}updateWorldMatrix(e,t,n=!1){super.updateWorldMatrix(e,t,n),this.matrixWorld.decompose(Vo,Go,Fn),Fn.x===1&&Fn.y===1&&Fn.z===1?this.matrixWorldInverse.copy(this.matrixWorld).invert():this.matrixWorldInverse.compose(Vo,Go,Fn.set(1,1,1)).invert()}clone(){return new this.constructor().copy(this)}},Mi=new L,Ou=new pe,Bu=new pe,Ct=class extends kr{constructor(e=50,t=1,n=.1,i=2e3){super(),this.isPerspectiveCamera=!0,this.type="PerspectiveCamera",this.fov=e,this.zoom=1,this.near=n,this.far=i,this.focus=10,this.aspect=t,this.view=null,this.filmGauge=35,this.filmOffset=0,this.updateProjectionMatrix()}copy(e,t){return super.copy(e,t),this.fov=e.fov,this.zoom=e.zoom,this.near=e.near,this.far=e.far,this.focus=e.focus,this.aspect=e.aspect,this.view=e.view===null?null:Object.assign({},e.view),this.filmGauge=e.filmGauge,this.filmOffset=e.filmOffset,this}setFocalLength(e){let t=.5*this.getFilmHeight()/e;this.fov=mr*2*Math.atan(t),this.updateProjectionMatrix()}getFocalLength(){let e=Math.tan(Nl*.5*this.fov);return .5*this.getFilmHeight()/e}getEffectiveFOV(){return mr*2*Math.atan(Math.tan(Nl*.5*this.fov)/this.zoom)}getFilmWidth(){return this.filmGauge*Math.min(this.aspect,1)}getFilmHeight(){return this.filmGauge/Math.max(this.aspect,1)}getViewBounds(e,t,n){Mi.set(-1,-1,.5).applyMatrix4(this.projectionMatrixInverse),t.set(Mi.x,Mi.y).multiplyScalar(-e/Mi.z),Mi.set(1,1,.5).applyMatrix4(this.projectionMatrixInverse),n.set(Mi.x,Mi.y).multiplyScalar(-e/Mi.z)}getViewSize(e,t){return this.getViewBounds(e,Ou,Bu),t.subVectors(Bu,Ou)}setViewOffset(e,t,n,i,r,o){this.aspect=e/t,this.view===null&&(this.view={enabled:!0,fullWidth:1,fullHeight:1,offsetX:0,offsetY:0,width:1,height:1}),this.view.enabled=!0,this.view.fullWidth=e,this.view.fullHeight=t,this.view.offsetX=n,this.view.offsetY=i,this.view.width=r,this.view.height=o,this.updateProjectionMatrix()}clearViewOffset(){this.view!==null&&(this.view.enabled=!1),this.updateProjectionMatrix()}updateProjectionMatrix(){let e=this.near,t=e*Math.tan(Nl*.5*this.fov)/this.zoom,n=2*t,i=this.aspect*n,r=-.5*i,o=this.view;if(this.view!==null&&this.view.enabled){let l=o.fullWidth,c=o.fullHeight;r+=o.offsetX*i/l,t-=o.offsetY*n/c,i*=o.width/l,n*=o.height/c}let a=this.filmOffset;a!==0&&(r+=e*a/this.getFilmWidth()),this.projectionMatrix.makePerspective(r,r+i,t,t-n,e,this.far,this.coordinateSystem,this.reversedDepth),this.projectionMatrixInverse.copy(this.projectionMatrix).invert()}toJSON(e){let t=super.toJSON(e);return t.object.fov=this.fov,t.object.zoom=this.zoom,t.object.near=this.near,t.object.far=this.far,t.object.focus=this.focus,t.object.aspect=this.aspect,this.view!==null&&(t.object.view=Object.assign({},this.view)),t.object.filmGauge=this.filmGauge,t.object.filmOffset=this.filmOffset,t}},_c=class extends zs{constructor(){super(new Ct(50,1,.5,500)),this.isSpotLightShadow=!0,this.focus=1,this.aspect=1}updateMatrices(e){let t=this.camera,n=mr*2*e.angle*this.focus,i=this.mapSize.width/this.mapSize.height*this.aspect,r=e.distance||t.far;(n!==t.fov||i!==t.aspect||r!==t.far)&&(t.fov=n,t.aspect=i,t.far=r,t.updateProjectionMatrix()),super.updateMatrices(e)}copy(e){return super.copy(e),this.focus=e.focus,this.aspect=e.aspect,this}toJSON(){let e=super.toJSON();return e.focus=this.focus,e.aspect=this.aspect,e}},Vr=class extends Ei{constructor(e,t,n=0,i=Math.PI/3,r=0,o=2){super(e,t),this.isSpotLight=!0,this.type="SpotLight",this.position.copy(At.DEFAULT_UP),this.updateMatrix(),this.target=new At,this.distance=n,this.angle=i,this.penumbra=r,this.decay=o,this.map=null,this.shadow=new _c}get power(){return this.intensity*Math.PI}set power(e){this.intensity=e/Math.PI}dispose(){super.dispose(),this.shadow.dispose()}copy(e,t){return super.copy(e,t),this.distance=e.distance,this.angle=e.angle,this.penumbra=e.penumbra,this.decay=e.decay,this.target=e.target.clone(),this.map=e.map,this.shadow=e.shadow.clone(),this}toJSON(e){let t=super.toJSON(e);return t.object.distance=this.distance,t.object.angle=this.angle,t.object.decay=this.decay,t.object.penumbra=this.penumbra,t.object.target=this.target.uuid,this.map&&this.map.isTexture&&(t.object.map=this.map.toJSON(e).uuid),t.object.shadow=this.shadow.toJSON(),t}},xc=class extends zs{constructor(){super(new Ct(90,1,.5,500)),this.isPointLightShadow=!0}},qi=class extends Ei{constructor(e,t,n=0,i=2){super(e,t),this.isPointLight=!0,this.type="PointLight",this.distance=n,this.decay=i,this.shadow=new xc}get power(){return this.intensity*4*Math.PI}set power(e){this.intensity=e/(4*Math.PI)}dispose(){super.dispose(),this.shadow.dispose()}copy(e,t){return super.copy(e,t),this.distance=e.distance,this.decay=e.decay,this.shadow=e.shadow.clone(),this}toJSON(e){let t=super.toJSON(e);return t.object.distance=this.distance,t.object.decay=this.decay,t.object.shadow=this.shadow.toJSON(),t}},Yi=class extends kr{constructor(e=-1,t=1,n=1,i=-1,r=.1,o=2e3){super(),this.isOrthographicCamera=!0,this.type="OrthographicCamera",this.zoom=1,this.view=null,this.left=e,this.right=t,this.top=n,this.bottom=i,this.near=r,this.far=o,this.updateProjectionMatrix()}copy(e,t){return super.copy(e,t),this.left=e.left,this.right=e.right,this.top=e.top,this.bottom=e.bottom,this.near=e.near,this.far=e.far,this.zoom=e.zoom,this.view=e.view===null?null:Object.assign({},e.view),this}setViewOffset(e,t,n,i,r,o){this.view===null&&(this.view={enabled:!0,fullWidth:1,fullHeight:1,offsetX:0,offsetY:0,width:1,height:1}),this.view.enabled=!0,this.view.fullWidth=e,this.view.fullHeight=t,this.view.offsetX=n,this.view.offsetY=i,this.view.width=r,this.view.height=o,this.updateProjectionMatrix()}clearViewOffset(){this.view!==null&&(this.view.enabled=!1),this.updateProjectionMatrix()}updateProjectionMatrix(){let e=(this.right-this.left)/(2*this.zoom),t=(this.top-this.bottom)/(2*this.zoom),n=(this.right+this.left)/2,i=(this.top+this.bottom)/2,r=n-e,o=n+e,a=i+t,l=i-t;if(this.view!==null&&this.view.enabled){let c=(this.right-this.left)/this.view.fullWidth/this.zoom,u=(this.top-this.bottom)/this.view.fullHeight/this.zoom;r+=c*this.view.offsetX,o=r+c*this.view.width,a-=u*this.view.offsetY,l=a-u*this.view.height}this.projectionMatrix.makeOrthographic(r,o,a,l,this.near,this.far,this.coordinateSystem,this.reversedDepth),this.projectionMatrixInverse.copy(this.projectionMatrix).invert()}toJSON(e){let t=super.toJSON(e);return t.object.zoom=this.zoom,t.object.left=this.left,t.object.right=this.right,t.object.top=this.top,t.object.bottom=this.bottom,t.object.near=this.near,t.object.far=this.far,this.view!==null&&(t.object.view=Object.assign({},this.view)),t}},yc=class extends zs{constructor(){super(new Yi(-5,5,5,-5,.5,500)),this.isDirectionalLightShadow=!0}},Sn=class extends Ei{constructor(e,t){super(e,t),this.isDirectionalLight=!0,this.type="DirectionalLight",this.position.copy(At.DEFAULT_UP),this.updateMatrix(),this.target=new At,this.shadow=new yc}dispose(){super.dispose(),this.shadow.dispose()}copy(e){return super.copy(e),this.target=e.target.clone(),this.shadow=e.shadow.clone(),this}toJSON(e){let t=super.toJSON(e);return t.object.shadow=this.shadow.toJSON(),t.object.target=this.target.uuid,t}},Zi=class extends Ei{constructor(e,t){super(e,t),this.isAmbientLight=!0,this.type="AmbientLight"}},ks=class extends Ei{constructor(e,t,n=10,i=10){super(e,t),this.isRectAreaLight=!0,this.type="RectAreaLight",this.width=n,this.height=i}get power(){return this.intensity*this.width*this.height*Math.PI}set power(e){this.intensity=e/(this.width*this.height*Math.PI)}copy(e){return super.copy(e),this.width=e.width,this.height=e.height,this}toJSON(e){let t=super.toJSON(e);return t.object.width=this.width,t.object.height=this.height,t}};var Ji=class{static extractUrlBase(e){let t=e.lastIndexOf("/");return t===-1?"./":e.slice(0,t+1)}static resolveURL(e,t){return typeof e!="string"||e===""?"":(/^https?:\/\//i.test(t)&&/^\//.test(e)&&(t=t.replace(/(^https?:\/\/[^\/]+).*/i,"$1")),/^(https?:)?\/\//i.test(e)||/^data:.*,.*$/i.test(e)||/^blob:.*$/i.test(e)?e:t+e)}};var ys=-90,vs=1,Vs=class extends At{constructor(e,t,n){super(),this.type="CubeCamera",this.renderTarget=n,this.coordinateSystem=null,this.activeMipmapLevel=0;let i=new Ct(ys,vs,e,t);i.layers=this.layers,this.add(i);let r=new Ct(ys,vs,e,t);r.layers=this.layers,this.add(r);let o=new Ct(ys,vs,e,t);o.layers=this.layers,this.add(o);let a=new Ct(ys,vs,e,t);a.layers=this.layers,this.add(a);let l=new Ct(ys,vs,e,t);l.layers=this.layers,this.add(l);let c=new Ct(ys,vs,e,t);c.layers=this.layers,this.add(c)}updateCoordinateSystem(){let e=this.coordinateSystem,t=this.children.concat(),[n,i,r,o,a,l]=t;for(let c of t)this.remove(c);if(e===xn)n.up.set(0,1,0),n.lookAt(1,0,0),i.up.set(0,1,0),i.lookAt(-1,0,0),r.up.set(0,0,-1),r.lookAt(0,1,0),o.up.set(0,0,1),o.lookAt(0,-1,0),a.up.set(0,1,0),a.lookAt(0,0,1),l.up.set(0,1,0),l.lookAt(0,0,-1);else if(e===As)n.up.set(0,-1,0),n.lookAt(-1,0,0),i.up.set(0,-1,0),i.lookAt(1,0,0),r.up.set(0,0,1),r.lookAt(0,1,0),o.up.set(0,0,-1),o.lookAt(0,-1,0),a.up.set(0,-1,0),a.lookAt(0,0,1),l.up.set(0,-1,0),l.lookAt(0,0,-1);else throw new Error("THREE.CubeCamera.updateCoordinateSystem(): Invalid coordinate system: "+e);for(let c of t)this.add(c),c.updateMatrixWorld()}update(e,t){this.parent===null&&this.updateMatrixWorld();let{renderTarget:n,activeMipmapLevel:i}=this;this.coordinateSystem!==e.coordinateSystem&&(this.coordinateSystem=e.coordinateSystem,this.updateCoordinateSystem());let[r,o,a,l,c,u]=this.children,h=e.getRenderTarget(),d=e.getActiveCubeFace(),f=e.getActiveMipmapLevel(),m=e.xr.enabled;e.xr.enabled=!1;let _=n.texture.mipmapsAutoUpdate;n.texture.mipmapsAutoUpdate=!1;let p=!1;e.isWebGLRenderer===!0?p=e.state.buffers.depth.getReversed():p=e.reversedDepthBuffer,e.setRenderTarget(n,0,i),p&&e.autoClear===!1&&e.clearDepth(),e.render(t,r),e.setRenderTarget(n,1,i),p&&e.autoClear===!1&&e.clearDepth(),e.render(t,o),e.setRenderTarget(n,2,i),p&&e.autoClear===!1&&e.clearDepth(),e.render(t,a),e.setRenderTarget(n,3,i),p&&e.autoClear===!1&&e.clearDepth(),e.render(t,l),e.setRenderTarget(n,4,i),p&&e.autoClear===!1&&e.clearDepth(),e.render(t,c),n.texture.mipmapsAutoUpdate=_,e.setRenderTarget(n,5,i),p&&e.autoClear===!1&&e.clearDepth(),e.render(t,u),e.setRenderTarget(h,d,f),e.xr.enabled=m,n.texture.needsPMREMUpdate=!0}},xa=class extends Ct{constructor(e=[]){super(),this.isArrayCamera=!0,this.isMultiViewCamera=!1,this.cameras=e,this.perCameraFrustumCulled=!1}};var Kc="\\[\\]\\.:\\/",Mm=new RegExp("["+Kc+"]","g"),jc="[^"+Kc+"]",bm="[^"+Kc.replace("\\.","")+"]",wm=/((?:WC+[\/:])*)/.source.replace("WC",jc),Am=/(WCOD+)?/.source.replace("WCOD",bm),Tm=/(?:\.(WC+)(?:\[(.+)\])?)?/.source.replace("WC",jc),Em=/\.(WC+)(?:\[(.+)\])?/.source.replace("WC",jc),Rm=new RegExp("^"+wm+Am+Tm+Em+"$"),Cm=["material","materials","bones","map"],vc=class{constructor(e,t,n){let i=n||bt.parseTrackName(t);this._targetGroup=e,this._bindings=e.subscribe_(t,i)}getValue(e,t){this.bind();let n=this._targetGroup.nCachedObjects_,i=this._bindings[n];i!==void 0&&i.getValue(e,t)}setValue(e,t){let n=this._bindings;for(let i=this._targetGroup.nCachedObjects_,r=n.length;i!==r;++i)n[i].setValue(e,t)}bind(){let e=this._bindings;for(let t=this._targetGroup.nCachedObjects_,n=e.length;t!==n;++t)e[t].bind()}unbind(){let e=this._bindings;for(let t=this._targetGroup.nCachedObjects_,n=e.length;t!==n;++t)e[t].unbind()}},bt=class s{constructor(e,t,n){this.path=t,this.parsedPath=n||s.parseTrackName(t),this.node=s.findNode(e,this.parsedPath.nodeName),this.rootNode=e,this.getValue=this._getValue_unbound,this.setValue=this._setValue_unbound}static create(e,t,n){return e&&e.isAnimationObjectGroup?new s.Composite(e,t,n):new s(e,t,n)}static sanitizeNodeName(e){return e.replace(/\s/g,"_").replace(Mm,"")}static parseTrackName(e){let t=Rm.exec(e);if(t===null)throw new Error("THREE.PropertyBinding: Cannot parse trackName: "+e);let n={nodeName:t[2],objectName:t[3],objectIndex:t[4],propertyName:t[5],propertyIndex:t[6]},i=n.nodeName&&n.nodeName.lastIndexOf(".");if(i!==void 0&&i!==-1){let r=n.nodeName.substring(i+1);Cm.indexOf(r)!==-1&&(n.nodeName=n.nodeName.substring(0,i),n.objectName=r)}if(n.propertyName===null||n.propertyName.length===0)throw new Error("THREE.PropertyBinding: can not parse propertyName from trackName: "+e);return n}static findNode(e,t){if(t===void 0||t===""||t==="."||t===-1||t===e.name||t===e.uuid)return e;if(e.skeleton){let n=e.skeleton.getBoneByName(t);if(n!==void 0)return n}if(e.children){let n=function(r){for(let o=0;o<r.length;o++){let a=r[o];if(a.name===t||a.uuid===t)return a;let l=n(a.children);if(l)return l}return null},i=n(e.children);if(i)return i}return null}_getValue_unavailable(){}_setValue_unavailable(){}_getValue_direct(e,t){e[t]=this.targetObject[this.propertyName]}_getValue_array(e,t){let n=this.resolvedProperty;for(let i=0,r=n.length;i!==r;++i)e[t++]=n[i]}_getValue_arrayElement(e,t){e[t]=this.resolvedProperty[this.propertyIndex]}_getValue_toArray(e,t){this.resolvedProperty.toArray(e,t)}_setValue_direct(e,t){this.targetObject[this.propertyName]=e[t]}_setValue_direct_setNeedsUpdate(e,t){this.targetObject[this.propertyName]=e[t],this.targetObject.needsUpdate=!0}_setValue_direct_setMatrixWorldNeedsUpdate(e,t){this.targetObject[this.propertyName]=e[t],this.targetObject.matrixWorldNeedsUpdate=!0}_setValue_array(e,t){let n=this.resolvedProperty;for(let i=0,r=n.length;i!==r;++i)n[i]=e[t++]}_setValue_array_setNeedsUpdate(e,t){let n=this.resolvedProperty;for(let i=0,r=n.length;i!==r;++i)n[i]=e[t++];this.targetObject.needsUpdate=!0}_setValue_array_setMatrixWorldNeedsUpdate(e,t){let n=this.resolvedProperty;for(let i=0,r=n.length;i!==r;++i)n[i]=e[t++];this.targetObject.matrixWorldNeedsUpdate=!0}_setValue_arrayElement(e,t){this.resolvedProperty[this.propertyIndex]=e[t]}_setValue_arrayElement_setNeedsUpdate(e,t){this.resolvedProperty[this.propertyIndex]=e[t],this.targetObject.needsUpdate=!0}_setValue_arrayElement_setMatrixWorldNeedsUpdate(e,t){this.resolvedProperty[this.propertyIndex]=e[t],this.targetObject.matrixWorldNeedsUpdate=!0}_setValue_fromArray(e,t){this.resolvedProperty.fromArray(e,t)}_setValue_fromArray_setNeedsUpdate(e,t){this.resolvedProperty.fromArray(e,t),this.targetObject.needsUpdate=!0}_setValue_fromArray_setMatrixWorldNeedsUpdate(e,t){this.resolvedProperty.fromArray(e,t),this.targetObject.matrixWorldNeedsUpdate=!0}_getValue_unbound(e,t){this.bind(),this.getValue(e,t)}_setValue_unbound(e,t){this.bind(),this.setValue(e,t)}bind(){let e=this.node,t=this.parsedPath,n=t.objectName,i=t.propertyName,r=t.propertyIndex;if(e||(e=s.findNode(this.rootNode,t.nodeName),this.node=e),this.getValue=this._getValue_unavailable,this.setValue=this._setValue_unavailable,!e){Xe("PropertyBinding: No target node found for track: "+this.path+".");return}if(n){let c=t.objectIndex;switch(n){case"materials":if(!e.material){Ye("PropertyBinding: Can not bind to material as node does not have a material.",this);return}if(!e.material.materials){Ye("PropertyBinding: Can not bind to material.materials as node.material does not have a materials array.",this);return}e=e.material.materials;break;case"bones":if(!e.skeleton){Ye("PropertyBinding: Can not bind to bones as node does not have a skeleton.",this);return}e=e.skeleton.bones;for(let u=0;u<e.length;u++)if(e[u].name===c){c=u;break}break;case"map":if("map"in e){e=e.map;break}if(!e.material){Ye("PropertyBinding: Can not bind to material as node does not have a material.",this);return}if(!e.material.map){Ye("PropertyBinding: Can not bind to material.map as node.material does not have a map.",this);return}e=e.material.map;break;default:if(e[n]===void 0){Ye("PropertyBinding: Can not bind to objectName of node undefined.",this);return}e=e[n]}if(c!==void 0){if(e[c]===void 0){Ye("PropertyBinding: Trying to bind to objectIndex of objectName, but is undefined.",this,e);return}e=e[c]}}let o=e[i];if(o===void 0){let c=t.nodeName;Ye("PropertyBinding: Trying to update property for track: "+c+"."+i+" but it wasn't found.",e);return}let a=this.Versioning.None;this.targetObject=e,e.isMaterial===!0?a=this.Versioning.NeedsUpdate:e.isObject3D===!0&&(a=this.Versioning.MatrixWorldNeedsUpdate);let l=this.BindingType.Direct;if(r!==void 0){if(i==="morphTargetInfluences"){if(!e.geometry){Ye("PropertyBinding: Can not bind to morphTargetInfluences because node does not have a geometry.",this);return}if(!e.geometry.morphAttributes){Ye("PropertyBinding: Can not bind to morphTargetInfluences because node does not have a geometry.morphAttributes.",this);return}e.morphTargetDictionary[r]!==void 0&&(r=e.morphTargetDictionary[r])}l=this.BindingType.ArrayElement,this.resolvedProperty=o,this.propertyIndex=r}else o.fromArray!==void 0&&o.toArray!==void 0?(l=this.BindingType.HasFromToArray,this.resolvedProperty=o):Array.isArray(o)?(l=this.BindingType.EntireArray,this.resolvedProperty=o):this.propertyName=i;this.getValue=this.GetterByBindingType[l],this.setValue=this.SetterByBindingTypeAndVersioning[l][a]}unbind(){this.node=null,this.getValue=this._getValue_unbound,this.setValue=this._setValue_unbound}};bt.Composite=vc;bt.prototype.BindingType={Direct:0,EntireArray:1,ArrayElement:2,HasFromToArray:3};bt.prototype.Versioning={None:0,NeedsUpdate:1,MatrixWorldNeedsUpdate:2};bt.prototype.GetterByBindingType=[bt.prototype._getValue_direct,bt.prototype._getValue_array,bt.prototype._getValue_arrayElement,bt.prototype._getValue_toArray];bt.prototype.SetterByBindingTypeAndVersioning=[[bt.prototype._setValue_direct,bt.prototype._setValue_direct_setNeedsUpdate,bt.prototype._setValue_direct_setMatrixWorldNeedsUpdate],[bt.prototype._setValue_array,bt.prototype._setValue_array_setNeedsUpdate,bt.prototype._setValue_array_setMatrixWorldNeedsUpdate],[bt.prototype._setValue_arrayElement,bt.prototype._setValue_arrayElement_setNeedsUpdate,bt.prototype._setValue_arrayElement_setMatrixWorldNeedsUpdate],[bt.prototype._setValue_fromArray,bt.prototype._setValue_fromArray_setNeedsUpdate,bt.prototype._setValue_fromArray_setMatrixWorldNeedsUpdate]];var yv=new Float32Array(1);var zu=new ze,ui=class{constructor(e,t,n=0,i=1/0){this.ray=new Is(e,t),this.near=n,this.far=i,this.camera=null,this.layers=new Cs,this.params={Mesh:{},Line:{threshold:1},LOD:{},Points:{threshold:1},Sprite:{}}}set(e,t){this.ray.set(e,t)}setFromCamera(e,t){t.isPerspectiveCamera?(this.ray.origin.setFromMatrixPosition(t.matrixWorld),this.ray.direction.set(e.x,e.y,.5).unproject(t).sub(this.ray.origin).normalize(),this.camera=t):t.isOrthographicCamera?(this.ray.origin.set(e.x,e.y,t.projectionMatrix.elements[14]).unproject(t),this.ray.direction.set(0,0,-1).transformDirection(t.matrixWorld),this.camera=t):Ye("Raycaster: Unsupported camera type: "+t.type)}setFromXRController(e){return zu.identity().extractRotation(e.matrixWorld),this.ray.origin.setFromMatrixPosition(e.matrixWorld),this.ray.direction.set(0,0,-1).applyMatrix4(zu),this}intersectObject(e,t=!0,n=[]){return Sc(e,this,n,t),n.sort(ku),n}intersectObjects(e,t=!0,n=[]){for(let i=0,r=e.length;i<r;i++)Sc(e[i],this,n,t);return n.sort(ku),n}};function ku(s,e){return s.distance-e.distance}function Sc(s,e,t,n){let i=!0;if(s.layers.test(e.layers)&&s.raycast(e,t)===!1&&(i=!1),i===!0&&n===!0){let r=s.children;for(let o=0,a=r.length;o<a;o++)Sc(r[o],e,t,!0)}}var sh=class sh{constructor(e,t,n,i){this.elements=[1,0,0,1],e!==void 0&&this.set(e,t,n,i)}identity(){return this.set(1,0,0,1),this}fromArray(e,t=0){for(let n=0;n<4;n++)this.elements[n]=e[n+t];return this}set(e,t,n,i){let r=this.elements;return r[0]=e,r[2]=t,r[1]=n,r[3]=i,this}};sh.prototype.isMatrix2=!0;var Mc=sh;function Qc(s,e,t,n){let i=Pm(n);switch(t){case Vc:return s*e;case Ea:return s*e/i.components*i.byteLength;case Ra:return s*e/i.components*i.byteLength;case Ci:return s*e*2/i.components*i.byteLength;case Ca:return s*e*2/i.components*i.byteLength;case Gc:return s*e*3/i.components*i.byteLength;case Mn:return s*e*4/i.components*i.byteLength;case Pa:return s*e*4/i.components*i.byteLength;case Wr:case Xr:return Math.floor((s+3)/4)*Math.floor((e+3)/4)*8;case qr:case Yr:return Math.floor((s+3)/4)*Math.floor((e+3)/4)*16;case Da:case Na:return Math.max(s,16)*Math.max(e,8)/4;case Ia:case La:return Math.max(s,8)*Math.max(e,8)/2;case Ua:case Fa:case Ba:case za:return Math.floor((s+3)/4)*Math.floor((e+3)/4)*8;case Oa:case Zr:case ka:return Math.floor((s+3)/4)*Math.floor((e+3)/4)*16;case Va:return Math.floor((s+3)/4)*Math.floor((e+3)/4)*16;case Ga:return Math.floor((s+4)/5)*Math.floor((e+3)/4)*16;case Ha:return Math.floor((s+4)/5)*Math.floor((e+4)/5)*16;case Wa:return Math.floor((s+5)/6)*Math.floor((e+4)/5)*16;case Xa:return Math.floor((s+5)/6)*Math.floor((e+5)/6)*16;case qa:return Math.floor((s+7)/8)*Math.floor((e+4)/5)*16;case Ya:return Math.floor((s+7)/8)*Math.floor((e+5)/6)*16;case Za:return Math.floor((s+7)/8)*Math.floor((e+7)/8)*16;case Ja:return Math.floor((s+9)/10)*Math.floor((e+4)/5)*16;case $a:return Math.floor((s+9)/10)*Math.floor((e+5)/6)*16;case Ka:return Math.floor((s+9)/10)*Math.floor((e+7)/8)*16;case ja:return Math.floor((s+9)/10)*Math.floor((e+9)/10)*16;case Qa:return Math.floor((s+11)/12)*Math.floor((e+9)/10)*16;case el:return Math.floor((s+11)/12)*Math.floor((e+11)/12)*16;case tl:case nl:case il:return Math.ceil(s/4)*Math.ceil(e/4)*16;case sl:case rl:return Math.ceil(s/4)*Math.ceil(e/4)*8;case Jr:case ol:return Math.ceil(s/4)*Math.ceil(e/4)*16}throw new Error(`Unable to determine texture byte length for ${t} format.`)}function Pm(s){switch(s){case on:case Oc:return{byteLength:1,components:1};case Xs:case Bc:case In:return{byteLength:2,components:1};case Aa:case Ta:return{byteLength:2,components:4};case Pn:case wa:case dn:return{byteLength:4,components:1};case zc:case kc:return{byteLength:4,components:3}}throw new Error(`THREE.TextureUtils: Unknown texture type ${s}.`)}typeof __THREE_DEVTOOLS__<"u"&&__THREE_DEVTOOLS__.dispatchEvent(new CustomEvent("register",{detail:{revision:"187"}}));typeof window<"u"&&(window.__THREE__?Xe("WARNING: Multiple instances of Three.js being imported."):window.__THREE__="187");function id(){let s=null,e=!1,t=null,n=null;function i(r,o){n=s.requestAnimationFrame(i),t(r,o)}return{start:function(){e!==!0&&t!==null&&s!==null&&(n=s.requestAnimationFrame(i),e=!0)},stop:function(){s!==null&&s.cancelAnimationFrame(n),e=!1},setAnimationLoop:function(r){t=r},setContext:function(r){s=r}}}function Bm(s){let e=new WeakMap;function t(a,l){let c=a.array,u=a.usage,h=c.byteLength,d=s.createBuffer();s.bindBuffer(l,d),s.bufferData(l,c,u),a.onUploadCallback();let f;if(c instanceof Float32Array)f=s.FLOAT;else if(typeof Float16Array<"u"&&c instanceof Float16Array)f=s.HALF_FLOAT;else if(c instanceof Uint16Array)a.isFloat16BufferAttribute?f=s.HALF_FLOAT:f=s.UNSIGNED_SHORT;else if(c instanceof Int16Array)f=s.SHORT;else if(c instanceof Uint32Array)f=s.UNSIGNED_INT;else if(c instanceof Int32Array)f=s.INT;else if(c instanceof Int8Array)f=s.BYTE;else if(c instanceof Uint8Array)f=s.UNSIGNED_BYTE;else if(c instanceof Uint8ClampedArray)f=s.UNSIGNED_BYTE;else throw new Error("THREE.WebGLAttributes: Unsupported buffer data format: "+c);return{buffer:d,type:f,bytesPerElement:c.BYTES_PER_ELEMENT,version:a.version,size:h}}function n(a,l,c){let u=l.array,h=l.updateRanges;if(s.bindBuffer(c,a),h.length===0)s.bufferSubData(c,0,u);else{h.sort((f,m)=>f.start-m.start);let d=0;for(let f=1;f<h.length;f++){let m=h[d],_=h[f];_.start<=m.start+m.count+1?m.count=Math.max(m.count,_.start+_.count-m.start):(++d,h[d]=_)}h.length=d+1;for(let f=0,m=h.length;f<m;f++){let _=h[f];s.bufferSubData(c,_.start*u.BYTES_PER_ELEMENT,u,_.start,_.count)}l.clearUpdateRanges()}l.onUploadCallback()}function i(a){return a.isInterleavedBufferAttribute&&(a=a.data),e.get(a)}function r(a){a.isInterleavedBufferAttribute&&(a=a.data);let l=e.get(a);l&&(s.deleteBuffer(l.buffer),e.delete(a))}function o(a,l){if(a.isInterleavedBufferAttribute&&(a=a.data),a.isGLBufferAttribute){let u=e.get(a);(!u||u.version<a.version)&&e.set(a,{buffer:a.buffer,type:a.type,bytesPerElement:a.elementSize,version:a.version});return}let c=e.get(a);if(c===void 0)e.set(a,t(a,l));else if(c.version<a.version){if(c.size!==a.array.byteLength)throw new Error("THREE.WebGLAttributes: The size of the buffer attribute's array buffer does not match the original size. Resizing buffer attributes is not supported.");n(c.buffer,a,l),c.version=a.version}}return{get:i,remove:r,update:o}}var zm=`#ifdef USE_ALPHAHASH
	if ( diffuseColor.a < getAlphaHashThreshold( vPosition ) ) discard;
#endif`,km=`#ifdef USE_ALPHAHASH
	const float ALPHA_HASH_SCALE = 0.05;
	float hash2D( vec2 value ) {
		return fract( 1.0e4 * sin( 17.0 * value.x + 0.1 * value.y ) * ( 0.1 + abs( sin( 13.0 * value.y + value.x ) ) ) );
	}
	float hash3D( vec3 value ) {
		return hash2D( vec2( hash2D( value.xy ), value.z ) );
	}
	float getAlphaHashThreshold( vec3 position ) {
		float maxDeriv = max(
			length( dFdx( position.xyz ) ),
			length( dFdy( position.xyz ) )
		);
		float pixScale = 1.0 / ( ALPHA_HASH_SCALE * maxDeriv );
		vec2 pixScales = vec2(
			exp2( floor( log2( pixScale ) ) ),
			exp2( ceil( log2( pixScale ) ) )
		);
		vec2 alpha = vec2(
			hash3D( floor( pixScales.x * position.xyz ) ),
			hash3D( floor( pixScales.y * position.xyz ) )
		);
		float lerpFactor = fract( log2( pixScale ) );
		float x = ( 1.0 - lerpFactor ) * alpha.x + lerpFactor * alpha.y;
		float a = min( lerpFactor, 1.0 - lerpFactor );
		vec3 cases = vec3(
			x * x / ( 2.0 * a * ( 1.0 - a ) ),
			( x - 0.5 * a ) / ( 1.0 - a ),
			1.0 - ( ( 1.0 - x ) * ( 1.0 - x ) / ( 2.0 * a * ( 1.0 - a ) ) )
		);
		float threshold = ( x < ( 1.0 - a ) )
			? ( ( x < a ) ? cases.x : cases.y )
			: cases.z;
		return clamp( threshold , 1.0e-6, 1.0 );
	}
#endif`,Vm=`#ifdef USE_ALPHAMAP
	diffuseColor.a *= texture2D( alphaMap, vAlphaMapUv ).g;
#endif`,Gm=`#ifdef USE_ALPHAMAP
	uniform sampler2D alphaMap;
#endif`,Hm=`#ifdef USE_ALPHATEST
	#ifdef ALPHA_TO_COVERAGE
	diffuseColor.a = smoothstep( alphaTest, alphaTest + fwidth( diffuseColor.a ), diffuseColor.a );
	if ( diffuseColor.a == 0.0 ) discard;
	#else
	if ( diffuseColor.a < alphaTest ) discard;
	#endif
#endif`,Wm=`#ifdef USE_ALPHATEST
	uniform float alphaTest;
#endif`,Xm=`#ifdef USE_AOMAP
	float ambientOcclusion = ( texture2D( aoMap, vAoMapUv ).r - 1.0 ) * aoMapIntensity + 1.0;
	reflectedLight.indirectDiffuse *= ambientOcclusion;
	#if defined( USE_CLEARCOAT ) 
		clearcoatSpecularIndirect *= ambientOcclusion;
	#endif
	#if defined( USE_SHEEN ) 
		sheenSpecularIndirect *= ambientOcclusion;
	#endif
	#if defined( USE_ENVMAP ) && defined( STANDARD )
		float dotNV = saturate( dot( geometryNormal, geometryViewDir ) );
		reflectedLight.indirectSpecular *= computeSpecularOcclusion( dotNV, ambientOcclusion, material.roughness );
	#endif
#endif`,qm=`#ifdef USE_AOMAP
	uniform sampler2D aoMap;
	uniform float aoMapIntensity;
#endif`,Ym=`#ifdef USE_BATCHING
	#if ! defined( GL_ANGLE_multi_draw )
	#define gl_DrawID _gl_DrawID
	uniform int _gl_DrawID;
	#endif
	uniform highp sampler2D batchingTexture;
	uniform highp usampler2D batchingIdTexture;
	mat4 getBatchingMatrix( const in float i ) {
		int size = textureSize( batchingTexture, 0 ).x;
		int j = int( i ) * 4;
		int x = j % size;
		int y = j / size;
		vec4 v1 = texelFetch( batchingTexture, ivec2( x, y ), 0 );
		vec4 v2 = texelFetch( batchingTexture, ivec2( x + 1, y ), 0 );
		vec4 v3 = texelFetch( batchingTexture, ivec2( x + 2, y ), 0 );
		vec4 v4 = texelFetch( batchingTexture, ivec2( x + 3, y ), 0 );
		return mat4( v1, v2, v3, v4 );
	}
	float getIndirectIndex( const in int i ) {
		int size = textureSize( batchingIdTexture, 0 ).x;
		int x = i % size;
		int y = i / size;
		return float( texelFetch( batchingIdTexture, ivec2( x, y ), 0 ).r );
	}
#endif
#ifdef USE_BATCHING_COLOR
	uniform sampler2D batchingColorTexture;
	vec4 getBatchingColor( const in float i ) {
		int size = textureSize( batchingColorTexture, 0 ).x;
		int j = int( i );
		int x = j % size;
		int y = j / size;
		return texelFetch( batchingColorTexture, ivec2( x, y ), 0 );
	}
#endif`,Zm=`#ifdef USE_BATCHING
	mat4 batchingMatrix = getBatchingMatrix( getIndirectIndex( gl_DrawID ) );
#endif`,Jm=`vec3 transformed = vec3( position );
#ifdef USE_ALPHAHASH
	vPosition = vec3( position );
#endif`,$m=`vec3 objectNormal = vec3( normal );
#ifdef USE_TANGENT
	vec3 objectTangent = vec3( tangent.xyz );
#endif`,Km=`float G_BlinnPhong_Implicit( ) {
	return 0.25;
}
float D_BlinnPhong( const in float shininess, const in float dotNH ) {
	return RECIPROCAL_PI * ( shininess * 0.5 + 1.0 ) * pow( dotNH, shininess );
}
vec3 BRDF_BlinnPhong( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, const in vec3 specularColor, const in float shininess ) {
	vec3 halfDir = normalize( lightDir + viewDir );
	float dotNH = saturate( dot( normal, halfDir ) );
	float dotVH = saturate( dot( viewDir, halfDir ) );
	vec3 F = F_Schlick( specularColor, 1.0, dotVH );
	float G = G_BlinnPhong_Implicit( );
	float D = D_BlinnPhong( shininess, dotNH );
	return F * ( G * D );
} // validated`,jm=`#ifdef USE_IRIDESCENCE
	const mat3 XYZ_TO_REC709 = mat3(
		 3.2404542, -0.9692660,  0.0556434,
		-1.5371385,  1.8760108, -0.2040259,
		-0.4985314,  0.0415560,  1.0572252
	);
	vec3 Fresnel0ToIor( vec3 fresnel0 ) {
		vec3 sqrtF0 = sqrt( fresnel0 );
		return ( vec3( 1.0 ) + sqrtF0 ) / ( vec3( 1.0 ) - sqrtF0 );
	}
	vec3 IorToFresnel0( vec3 transmittedIor, float incidentIor ) {
		return pow2( ( transmittedIor - vec3( incidentIor ) ) / ( transmittedIor + vec3( incidentIor ) ) );
	}
	float IorToFresnel0( float transmittedIor, float incidentIor ) {
		return pow2( ( transmittedIor - incidentIor ) / ( transmittedIor + incidentIor ));
	}
	vec3 evalSensitivity( float OPD, vec3 shift ) {
		float phase = 2.0 * PI * OPD * 1.0e-9;
		vec3 val = vec3( 5.4856e-13, 4.4201e-13, 5.2481e-13 );
		vec3 pos = vec3( 1.6810e+06, 1.7953e+06, 2.2084e+06 );
		vec3 var = vec3( 4.3278e+09, 9.3046e+09, 6.6121e+09 );
		vec3 xyz = val * sqrt( 2.0 * PI * var ) * cos( pos * phase + shift ) * exp( - pow2( phase ) * var );
		xyz.x += 9.7470e-14 * sqrt( 2.0 * PI * 4.5282e+09 ) * cos( 2.2399e+06 * phase + shift[ 0 ] ) * exp( - 4.5282e+09 * pow2( phase ) );
		xyz /= 1.0685e-7;
		vec3 rgb = XYZ_TO_REC709 * xyz;
		return rgb;
	}
	vec3 evalIridescence( float outsideIOR, float eta2, float cosTheta1, float thinFilmThickness, vec3 baseF0 ) {
		vec3 I;
		float iridescenceIOR = mix( outsideIOR, eta2, smoothstep( 0.0, 0.03, thinFilmThickness ) );
		float sinTheta2Sq = pow2( outsideIOR / iridescenceIOR ) * ( 1.0 - pow2( cosTheta1 ) );
		float cosTheta2Sq = 1.0 - sinTheta2Sq;
		if ( cosTheta2Sq < 0.0 ) {
			return vec3( 1.0 );
		}
		float cosTheta2 = sqrt( cosTheta2Sq );
		float R0 = IorToFresnel0( iridescenceIOR, outsideIOR );
		float R12 = F_Schlick( R0, 1.0, cosTheta1 );
		float T121 = 1.0 - R12;
		float phi12 = 0.0;
		if ( iridescenceIOR < outsideIOR ) phi12 = PI;
		float phi21 = PI - phi12;
		vec3 baseIOR = Fresnel0ToIor( clamp( baseF0, 0.0, 0.9999 ) );		vec3 R1 = IorToFresnel0( baseIOR, iridescenceIOR );
		vec3 R23 = F_Schlick( R1, 1.0, cosTheta2 );
		vec3 phi23 = vec3( 0.0 );
		if ( baseIOR[ 0 ] < iridescenceIOR ) phi23[ 0 ] = PI;
		if ( baseIOR[ 1 ] < iridescenceIOR ) phi23[ 1 ] = PI;
		if ( baseIOR[ 2 ] < iridescenceIOR ) phi23[ 2 ] = PI;
		float OPD = 2.0 * iridescenceIOR * thinFilmThickness * cosTheta2;
		vec3 phi = vec3( phi21 ) + phi23;
		vec3 R123 = clamp( R12 * R23, 1e-5, 0.9999 );
		vec3 r123 = sqrt( R123 );
		vec3 Rs = pow2( T121 ) * R23 / ( vec3( 1.0 ) - R123 );
		vec3 C0 = R12 + Rs;
		I = C0;
		vec3 Cm = Rs - T121;
		for ( int m = 1; m <= 2; ++ m ) {
			Cm *= r123;
			vec3 Sm = 2.0 * evalSensitivity( float( m ) * OPD, float( m ) * phi );
			I += Cm * Sm;
		}
		return max( I, vec3( 0.0 ) );
	}
#endif`,Qm=`#ifdef USE_BUMPMAP
	uniform sampler2D bumpMap;
	uniform float bumpScale;
	vec2 dHdxy_fwd() {
		vec2 dSTdx = dFdx( vBumpMapUv );
		vec2 dSTdy = dFdy( vBumpMapUv );
		float Hll = bumpScale * texture2D( bumpMap, vBumpMapUv ).x;
		float dBx = bumpScale * texture2D( bumpMap, vBumpMapUv + dSTdx ).x - Hll;
		float dBy = bumpScale * texture2D( bumpMap, vBumpMapUv + dSTdy ).x - Hll;
		return vec2( dBx, dBy );
	}
	vec3 perturbNormalArb( vec3 surf_pos, vec3 surf_norm, vec2 dHdxy, float faceDirection ) {
		vec3 vSigmaX = normalize( dFdx( surf_pos.xyz ) );
		vec3 vSigmaY = normalize( dFdy( surf_pos.xyz ) );
		vec3 vN = surf_norm;
		vec3 R1 = cross( vSigmaY, vN );
		vec3 R2 = cross( vN, vSigmaX );
		float fDet = dot( vSigmaX, R1 ) * faceDirection;
		vec3 vGrad = sign( fDet ) * ( dHdxy.x * R1 + dHdxy.y * R2 );
		return normalize( abs( fDet ) * surf_norm - vGrad );
	}
#endif`,eg=`#if NUM_CLIPPING_PLANES > 0
	vec4 plane;
	#ifdef ALPHA_TO_COVERAGE
		float distanceToPlane, distanceGradient;
		float clipOpacity = 1.0;
		#pragma unroll_loop_start
		for ( int i = 0; i < UNION_CLIPPING_PLANES; i ++ ) {
			plane = clippingPlanes[ i ];
			distanceToPlane = - dot( vClipPosition, plane.xyz ) + plane.w;
			distanceGradient = fwidth( distanceToPlane ) / 2.0;
			clipOpacity *= smoothstep( - distanceGradient, distanceGradient, distanceToPlane );
			if ( clipOpacity == 0.0 ) discard;
		}
		#pragma unroll_loop_end
		#if UNION_CLIPPING_PLANES < NUM_CLIPPING_PLANES
			float unionClipOpacity = 1.0;
			#pragma unroll_loop_start
			for ( int i = UNION_CLIPPING_PLANES; i < NUM_CLIPPING_PLANES; i ++ ) {
				plane = clippingPlanes[ i ];
				distanceToPlane = - dot( vClipPosition, plane.xyz ) + plane.w;
				distanceGradient = fwidth( distanceToPlane ) / 2.0;
				unionClipOpacity *= 1.0 - smoothstep( - distanceGradient, distanceGradient, distanceToPlane );
			}
			#pragma unroll_loop_end
			clipOpacity *= 1.0 - unionClipOpacity;
		#endif
		diffuseColor.a *= clipOpacity;
		if ( diffuseColor.a == 0.0 ) discard;
	#else
		#pragma unroll_loop_start
		for ( int i = 0; i < UNION_CLIPPING_PLANES; i ++ ) {
			plane = clippingPlanes[ i ];
			if ( dot( vClipPosition, plane.xyz ) > plane.w ) discard;
		}
		#pragma unroll_loop_end
		#if UNION_CLIPPING_PLANES < NUM_CLIPPING_PLANES
			bool clipped = true;
			#pragma unroll_loop_start
			for ( int i = UNION_CLIPPING_PLANES; i < NUM_CLIPPING_PLANES; i ++ ) {
				plane = clippingPlanes[ i ];
				clipped = ( dot( vClipPosition, plane.xyz ) > plane.w ) && clipped;
			}
			#pragma unroll_loop_end
			if ( clipped ) discard;
		#endif
	#endif
#endif`,tg=`#if NUM_CLIPPING_PLANES > 0
	varying vec3 vClipPosition;
	uniform vec4 clippingPlanes[ NUM_CLIPPING_PLANES ];
#endif`,ng=`#if NUM_CLIPPING_PLANES > 0
	varying vec3 vClipPosition;
#endif`,ig=`#if NUM_CLIPPING_PLANES > 0
	vClipPosition = - mvPosition.xyz;
#endif`,sg=`#if defined( USE_COLOR ) || defined( USE_COLOR_ALPHA )
	diffuseColor *= vColor;
#endif`,rg=`#if defined( USE_COLOR ) || defined( USE_COLOR_ALPHA )
	varying vec4 vColor;
#endif`,og=`#if defined( USE_COLOR ) || defined( USE_COLOR_ALPHA ) || defined( USE_INSTANCING_COLOR ) || defined( USE_BATCHING_COLOR )
	varying vec4 vColor;
#endif`,ag=`#if defined( USE_COLOR ) || defined( USE_COLOR_ALPHA ) || defined( USE_INSTANCING_COLOR ) || defined( USE_BATCHING_COLOR )
	vColor = vec4( 1.0 );
#endif
#ifdef USE_COLOR_ALPHA
	vColor *= color;
#elif defined( USE_COLOR )
	vColor.rgb *= color;
#endif
#ifdef USE_INSTANCING_COLOR
	vColor.rgb *= instanceColor.rgb;
#endif
#ifdef USE_BATCHING_COLOR
	vColor *= getBatchingColor( getIndirectIndex( gl_DrawID ) );
#endif`,lg=`#define PI 3.141592653589793
#define PI2 6.283185307179586
#define PI_HALF 1.5707963267948966
#define RECIPROCAL_PI 0.3183098861837907
#define RECIPROCAL_PI2 0.15915494309189535
#define EPSILON 1e-6
#ifndef saturate
#define saturate( a ) clamp( a, 0.0, 1.0 )
#endif
#define whiteComplement( a ) ( 1.0 - saturate( a ) )
float pow2( const in float x ) { return x*x; }
vec3 pow2( const in vec3 x ) { return x*x; }
float pow3( const in float x ) { return x*x*x; }
float pow4( const in float x ) { float x2 = x*x; return x2*x2; }
float max3( const in vec3 v ) { return max( max( v.x, v.y ), v.z ); }
float average( const in vec3 v ) { return dot( v, vec3( 0.3333333 ) ); }
highp float rand( const in vec2 uv ) {
	const highp float a = 12.9898, b = 78.233, c = 43758.5453;
	highp float dt = dot( uv.xy, vec2( a,b ) ), sn = mod( dt, PI );
	return fract( sin( sn ) * c );
}
#ifdef HIGH_PRECISION
	float precisionSafeLength( vec3 v ) { return length( v ); }
#else
	float precisionSafeLength( vec3 v ) {
		float maxComponent = max3( abs( v ) );
		return length( v / maxComponent ) * maxComponent;
	}
#endif
struct IncidentLight {
	vec3 color;
	vec3 direction;
	bool visible;
};
struct ReflectedLight {
	vec3 directDiffuse;
	vec3 directSpecular;
	vec3 indirectDiffuse;
	vec3 indirectSpecular;
};
#ifdef USE_ALPHAHASH
	varying vec3 vPosition;
#endif
vec3 transformDirection( in vec3 dir, in mat4 matrix ) {
	return normalize( ( matrix * vec4( dir, 0.0 ) ).xyz );
}
#define inverseTransformDirection transformDirectionByInverseViewMatrix
vec3 transformNormalByInverseViewMatrix( in vec3 normal, in mat4 viewMatrix ) {
	return normalize( ( vec4( normal, 0.0 ) * viewMatrix ).xyz );
}
vec3 transformDirectionByInverseViewMatrix( in vec3 dir, in mat4 viewMatrix ) {
	return normalize( ( vec4( dir, 0.0 ) * viewMatrix ).xyz );
}
bool isPerspectiveMatrix( mat4 m ) {
	return m[ 2 ][ 3 ] == - 1.0;
}
vec2 equirectUv( in vec3 dir ) {
	float u = atan( dir.z, dir.x ) * RECIPROCAL_PI2 + 0.5;
	float v = asin( clamp( dir.y, - 1.0, 1.0 ) ) * RECIPROCAL_PI + 0.5;
	return vec2( u, v );
}
vec3 BRDF_Lambert( const in vec3 diffuseColor ) {
	return RECIPROCAL_PI * diffuseColor;
}
vec3 F_Schlick( const in vec3 f0, const in float f90, const in float dotVH ) {
	float fresnel = exp2( ( - 5.55473 * dotVH - 6.98316 ) * dotVH );
	return f0 * ( 1.0 - fresnel ) + ( f90 * fresnel );
}
float F_Schlick( const in float f0, const in float f90, const in float dotVH ) {
	float fresnel = exp2( ( - 5.55473 * dotVH - 6.98316 ) * dotVH );
	return f0 * ( 1.0 - fresnel ) + ( f90 * fresnel );
} // validated`,cg=`vec3 transformedNormal = objectNormal;
#ifdef USE_TANGENT
	vec3 transformedTangent = objectTangent;
#endif
#ifdef USE_BATCHING
	mat3 bm = mat3( batchingMatrix );
	transformedNormal /= vec3( dot( bm[ 0 ], bm[ 0 ] ), dot( bm[ 1 ], bm[ 1 ] ), dot( bm[ 2 ], bm[ 2 ] ) );
	transformedNormal = bm * transformedNormal;
	#ifdef USE_TANGENT
		transformedTangent = bm * transformedTangent;
	#endif
#endif
#ifdef USE_INSTANCING
	mat3 im = mat3( instanceMatrix );
	transformedNormal /= vec3( dot( im[ 0 ], im[ 0 ] ), dot( im[ 1 ], im[ 1 ] ), dot( im[ 2 ], im[ 2 ] ) );
	transformedNormal = im * transformedNormal;
	#ifdef USE_TANGENT
		transformedTangent = im * transformedTangent;
	#endif
#endif
transformedNormal = normalMatrix * transformedNormal;
#ifdef FLIP_SIDED
	transformedNormal = - transformedNormal;
#endif
#ifdef USE_TANGENT
	transformedTangent = ( modelViewMatrix * vec4( transformedTangent, 0.0 ) ).xyz;
#endif`,hg=`#ifdef USE_DISPLACEMENTMAP
	uniform sampler2D displacementMap;
	uniform float displacementScale;
	uniform float displacementBias;
#endif`,ug=`#ifdef USE_DISPLACEMENTMAP
	transformed += normalize( objectNormal ) * ( texture2D( displacementMap, vDisplacementMapUv ).x * displacementScale + displacementBias );
#endif`,fg=`#ifdef USE_EMISSIVEMAP
	vec4 emissiveColor = texture2D( emissiveMap, vEmissiveMapUv );
	#ifdef DECODE_VIDEO_TEXTURE_EMISSIVE
		emissiveColor = sRGBTransferEOTF( emissiveColor );
	#endif
	totalEmissiveRadiance *= emissiveColor.rgb;
#endif`,dg=`#ifdef USE_EMISSIVEMAP
	uniform sampler2D emissiveMap;
#endif`,pg="gl_FragColor = linearToOutputTexel( gl_FragColor );",mg=`vec4 LinearTransferOETF( in vec4 value ) {
	return value;
}
vec4 sRGBTransferEOTF( in vec4 value ) {
	return vec4( mix( pow( value.rgb * 0.9478672986 + vec3( 0.0521327014 ), vec3( 2.4 ) ), value.rgb * 0.0773993808, vec3( lessThanEqual( value.rgb, vec3( 0.04045 ) ) ) ), value.a );
}
vec4 sRGBTransferOETF( in vec4 value ) {
	return vec4( mix( pow( value.rgb, vec3( 0.41666 ) ) * 1.055 - vec3( 0.055 ), value.rgb * 12.92, vec3( lessThanEqual( value.rgb, vec3( 0.0031308 ) ) ) ), value.a );
}`,gg=`#ifdef USE_ENVMAP
	#ifdef ENV_WORLDPOS
		vec3 cameraToFrag;
		if ( isOrthographic ) {
			cameraToFrag = normalize( vec3( - viewMatrix[ 0 ][ 2 ], - viewMatrix[ 1 ][ 2 ], - viewMatrix[ 2 ][ 2 ] ) );
		} else {
			cameraToFrag = normalize( vWorldPosition - cameraPosition );
		}
		vec3 worldNormal = transformNormalByInverseViewMatrix( normal, viewMatrix );
		#ifdef ENVMAP_MODE_REFLECTION
			vec3 reflectVec = reflect( cameraToFrag, worldNormal );
		#else
			vec3 reflectVec = refract( cameraToFrag, worldNormal, refractionRatio );
		#endif
	#else
		vec3 reflectVec = vReflect;
	#endif
	#ifdef ENVMAP_TYPE_CUBE
		vec4 envColor = textureCube( envMap, envMapRotation * reflectVec );
		#ifdef ENVMAP_BLENDING_MULTIPLY
			outgoingLight = mix( outgoingLight, outgoingLight * envColor.xyz, specularStrength * reflectivity );
		#elif defined( ENVMAP_BLENDING_MIX )
			outgoingLight = mix( outgoingLight, envColor.xyz, specularStrength * reflectivity );
		#elif defined( ENVMAP_BLENDING_ADD )
			outgoingLight += envColor.xyz * specularStrength * reflectivity;
		#endif
	#endif
#endif`,_g=`#ifdef USE_ENVMAP
	uniform float envMapIntensity;
	uniform mat3 envMapRotation;
	uniform samplerCube envMap;
	#ifdef ENVMAP_TYPE_PMREM
		float roughnessToMip( const in float roughness ) {
			float r = clamp( roughness, 0.0, 1.0 );
			return ENVMAP_MAX_LOD * r * ( 2.0 - r );
		}
	#endif
#endif`,xg=`#ifdef USE_ENVMAP
	uniform float reflectivity;
	#if defined( USE_BUMPMAP ) || defined( USE_NORMALMAP ) || defined( PHONG ) || defined( LAMBERT )
		#define ENV_WORLDPOS
	#endif
	#ifdef ENV_WORLDPOS
		varying vec3 vWorldPosition;
		uniform float refractionRatio;
	#else
		varying vec3 vReflect;
	#endif
#endif`,yg=`#ifdef USE_ENVMAP
	#if defined( USE_BUMPMAP ) || defined( USE_NORMALMAP ) || defined( PHONG ) || defined( LAMBERT )
		#define ENV_WORLDPOS
	#endif
	#ifdef ENV_WORLDPOS
		
		varying vec3 vWorldPosition;
	#else
		varying vec3 vReflect;
		uniform float refractionRatio;
	#endif
#endif`,vg=`#ifdef USE_ENVMAP
	#ifdef ENV_WORLDPOS
		vWorldPosition = worldPosition.xyz;
	#else
		vec3 cameraToVertex;
		if ( isOrthographic ) {
			cameraToVertex = normalize( vec3( - viewMatrix[ 0 ][ 2 ], - viewMatrix[ 1 ][ 2 ], - viewMatrix[ 2 ][ 2 ] ) );
		} else {
			cameraToVertex = normalize( worldPosition.xyz - cameraPosition );
		}
		vec3 worldNormal = transformNormalByInverseViewMatrix( transformedNormal, viewMatrix );
		#ifdef ENVMAP_MODE_REFLECTION
			vReflect = reflect( cameraToVertex, worldNormal );
		#else
			vReflect = refract( cameraToVertex, worldNormal, refractionRatio );
		#endif
	#endif
#endif`,Sg=`#ifdef USE_FOG
	vFogDepth = - mvPosition.z;
#endif`,Mg=`#ifdef USE_FOG
	varying float vFogDepth;
#endif`,bg=`#ifdef USE_FOG
	#ifdef FOG_EXP2
		float fogFactor = 1.0 - exp( - fogDensity * fogDensity * vFogDepth * vFogDepth );
	#else
		float fogFactor = smoothstep( fogNear, fogFar, vFogDepth );
	#endif
	gl_FragColor.rgb = mix( gl_FragColor.rgb, fogColor, fogFactor );
#endif`,wg=`#ifdef USE_FOG
	uniform vec3 fogColor;
	varying float vFogDepth;
	#ifdef FOG_EXP2
		uniform float fogDensity;
	#else
		uniform float fogNear;
		uniform float fogFar;
	#endif
#endif`,Ag=`#ifdef USE_GRADIENTMAP
	uniform sampler2D gradientMap;
#endif
vec3 getGradientIrradiance( vec3 normal, vec3 lightDirection ) {
	float dotNL = dot( normal, lightDirection );
	vec2 coord = vec2( dotNL * 0.5 + 0.5, 0.0 );
	#ifdef USE_GRADIENTMAP
		return vec3( texture2D( gradientMap, coord ).r );
	#else
		vec2 fw = fwidth( coord ) * 0.5;
		return mix( vec3( 0.7 ), vec3( 1.0 ), smoothstep( 0.7 - fw.x, 0.7 + fw.x, coord.x ) );
	#endif
}`,Tg=`#ifdef USE_LIGHTMAP
	uniform sampler2D lightMap;
	uniform float lightMapIntensity;
#endif`,Eg=`LambertMaterial material;
material.diffuseColor = diffuseColor.rgb;
material.specularStrength = specularStrength;`,Rg=`varying vec3 vViewPosition;
struct LambertMaterial {
	vec3 diffuseColor;
	float specularStrength;
};
void RE_Direct_Lambert( const in IncidentLight directLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in LambertMaterial material, inout ReflectedLight reflectedLight ) {
	float dotNL = saturate( dot( geometryNormal, directLight.direction ) );
	vec3 irradiance = dotNL * directLight.color;
	reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
}
void RE_IndirectDiffuse_Lambert( const in vec3 irradiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in LambertMaterial material, inout ReflectedLight reflectedLight ) {
	reflectedLight.indirectDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
}
#define RE_Direct				RE_Direct_Lambert
#define RE_IndirectDiffuse		RE_IndirectDiffuse_Lambert`,Cg=`uniform bool receiveShadow;
uniform vec3 ambientLightColor;
#if defined( USE_LIGHT_PROBES )
	uniform vec3 lightProbe[ 9 ];
#endif
vec3 shGetIrradianceAt( in vec3 normal, in vec3 shCoefficients[ 9 ] ) {
	float x = normal.x, y = normal.y, z = normal.z;
	vec3 result = shCoefficients[ 0 ] * 0.886227;
	result += shCoefficients[ 1 ] * 2.0 * 0.511664 * y;
	result += shCoefficients[ 2 ] * 2.0 * 0.511664 * z;
	result += shCoefficients[ 3 ] * 2.0 * 0.511664 * x;
	result += shCoefficients[ 4 ] * 2.0 * 0.429043 * x * y;
	result += shCoefficients[ 5 ] * 2.0 * 0.429043 * y * z;
	result += shCoefficients[ 6 ] * ( 0.743125 * z * z - 0.247708 );
	result += shCoefficients[ 7 ] * 2.0 * 0.429043 * x * z;
	result += shCoefficients[ 8 ] * 0.429043 * ( x * x - y * y );
	return result;
}
vec3 getLightProbeIrradiance( const in vec3 lightProbe[ 9 ], const in vec3 normal ) {
	vec3 worldNormal = transformNormalByInverseViewMatrix( normal, viewMatrix );
	vec3 irradiance = shGetIrradianceAt( worldNormal, lightProbe );
	return irradiance;
}
vec3 getAmbientLightIrradiance( const in vec3 ambientLightColor ) {
	vec3 irradiance = ambientLightColor;
	return irradiance;
}
float getDistanceAttenuation( const in float lightDistance, const in float cutoffDistance, const in float decayExponent ) {
	float distanceFalloff = 1.0 / max( pow( lightDistance, decayExponent ), 0.01 );
	if ( cutoffDistance > 0.0 ) {
		distanceFalloff *= pow2( saturate( 1.0 - pow4( lightDistance / cutoffDistance ) ) );
	}
	return distanceFalloff;
}
float getSpotAttenuation( const in float coneCosine, const in float penumbraCosine, const in float angleCosine ) {
	return smoothstep( coneCosine, penumbraCosine, angleCosine );
}
#if NUM_SUN_LIGHTS > 0
	struct SunLight {
		vec3 direction;
		vec3 color;
	};
	uniform SunLight sunLights[ NUM_SUN_LIGHTS ];
	void getSunLightInfo( const in SunLight sunLight, out IncidentLight light ) {
		light.color = sunLight.color;
		light.direction = sunLight.direction;
		light.visible = true;
	}
#endif
#if NUM_DIR_LIGHTS > 0
	struct DirectionalLight {
		vec3 direction;
		vec3 color;
	};
	uniform DirectionalLight directionalLights[ NUM_DIR_LIGHTS ];
	void getDirectionalLightInfo( const in DirectionalLight directionalLight, out IncidentLight light ) {
		light.color = directionalLight.color;
		light.direction = directionalLight.direction;
		light.visible = true;
	}
#endif
#if NUM_POINT_LIGHTS > 0
	struct PointLight {
		vec3 position;
		vec3 color;
		float distance;
		float decay;
	};
	uniform PointLight pointLights[ NUM_POINT_LIGHTS ];
	void getPointLightInfo( const in PointLight pointLight, const in vec3 geometryPosition, out IncidentLight light ) {
		vec3 lVector = pointLight.position - geometryPosition;
		light.direction = normalize( lVector );
		float lightDistance = length( lVector );
		light.color = pointLight.color;
		light.color *= getDistanceAttenuation( lightDistance, pointLight.distance, pointLight.decay );
		light.visible = ( light.color != vec3( 0.0 ) );
	}
#endif
#if NUM_SPOT_LIGHTS > 0
	struct SpotLight {
		vec3 position;
		vec3 direction;
		vec3 color;
		float distance;
		float decay;
		float coneCos;
		float penumbraCos;
	};
	uniform SpotLight spotLights[ NUM_SPOT_LIGHTS ];
	void getSpotLightInfo( const in SpotLight spotLight, const in vec3 geometryPosition, out IncidentLight light ) {
		vec3 lVector = spotLight.position - geometryPosition;
		light.direction = normalize( lVector );
		float angleCos = dot( light.direction, spotLight.direction );
		float spotAttenuation = getSpotAttenuation( spotLight.coneCos, spotLight.penumbraCos, angleCos );
		if ( spotAttenuation > 0.0 ) {
			float lightDistance = length( lVector );
			light.color = spotLight.color * spotAttenuation;
			light.color *= getDistanceAttenuation( lightDistance, spotLight.distance, spotLight.decay );
			light.visible = ( light.color != vec3( 0.0 ) );
		} else {
			light.color = vec3( 0.0 );
			light.visible = false;
		}
	}
#endif
#if NUM_RECT_AREA_LIGHTS > 0
	struct RectAreaLight {
		vec3 color;
		vec3 position;
		vec3 halfWidth;
		vec3 halfHeight;
	};
	uniform sampler2D ltc_1;	uniform sampler2D ltc_2;
	uniform RectAreaLight rectAreaLights[ NUM_RECT_AREA_LIGHTS ];
#endif
#if NUM_HEMI_LIGHTS > 0
	struct HemisphereLight {
		vec3 direction;
		vec3 skyColor;
		vec3 groundColor;
	};
	uniform HemisphereLight hemisphereLights[ NUM_HEMI_LIGHTS ];
	vec3 getHemisphereLightIrradiance( const in HemisphereLight hemiLight, const in vec3 normal ) {
		float dotNL = dot( normal, hemiLight.direction );
		float hemiDiffuseWeight = 0.5 * dotNL + 0.5;
		vec3 irradiance = mix( hemiLight.groundColor, hemiLight.skyColor, hemiDiffuseWeight );
		return irradiance;
	}
#endif
#include <lightprobes_pars_fragment>`,Pg=`#ifdef USE_ENVMAP
	vec3 getIBLIrradiance( const in vec3 normal ) {
		#ifdef ENVMAP_TYPE_PMREM
			vec3 worldNormal = transformNormalByInverseViewMatrix( normal, viewMatrix );
			vec4 envMapColor = textureLod( envMap, envMapRotation * worldNormal, ENVMAP_MAX_LOD );
			return PI * envMapColor.rgb * envMapIntensity;
		#else
			return vec3( 0.0 );
		#endif
	}
	vec3 getIBLRadiance( const in vec3 viewDir, const in vec3 normal, const in float roughness ) {
		#ifdef ENVMAP_TYPE_PMREM
			vec3 reflectVec = reflect( - viewDir, normal );
			reflectVec = normalize( mix( reflectVec, normal, pow4( roughness ) ) );
			reflectVec = transformDirectionByInverseViewMatrix( reflectVec, viewMatrix );
			vec4 envMapColor = textureLod( envMap, envMapRotation * reflectVec, roughnessToMip( roughness ) );
			return envMapColor.rgb * envMapIntensity;
		#else
			return vec3( 0.0 );
		#endif
	}
	#ifdef USE_RETROREFLECTION
		vec3 getIBLRetroRadiance( const in vec3 viewDir, const in vec3 normal, const in float roughness ) {
			#ifdef ENVMAP_TYPE_PMREM
				vec3 retroVec = normalize( mix( viewDir, normal, pow4( roughness ) ) );
				retroVec = transformDirectionByInverseViewMatrix( retroVec, viewMatrix );
				vec4 envMapColor = textureLod( envMap, envMapRotation * retroVec, roughnessToMip( roughness ) );
				return envMapColor.rgb * envMapIntensity;
			#else
				return vec3( 0.0 );
			#endif
		}
	#endif
	#ifdef USE_ANISOTROPY
		vec3 getIBLAnisotropyRadiance( const in vec3 viewDir, const in vec3 normal, const in float roughness, const in vec3 bitangent, const in float anisotropy ) {
			#ifdef ENVMAP_TYPE_PMREM
				vec3 bentNormal = cross( bitangent, viewDir );
				bentNormal = normalize( cross( bentNormal, bitangent ) );
				bentNormal = normalize( mix( bentNormal, normal, pow2( pow2( 1.0 - anisotropy * ( 1.0 - roughness ) ) ) ) );
				return getIBLRadiance( viewDir, bentNormal, roughness );
			#else
				return vec3( 0.0 );
			#endif
		}
		#ifdef USE_RETROREFLECTION
			vec3 getIBLAnisotropyRetroRadiance( const in vec3 viewDir, const in vec3 normal, const in float roughness, const in vec3 bitangent, const in float anisotropy ) {
				#ifdef ENVMAP_TYPE_PMREM
					vec3 bentNormal = cross( bitangent, viewDir );
					bentNormal = normalize( cross( bentNormal, bitangent ) );
					bentNormal = normalize( mix( bentNormal, normal, pow2( pow2( 1.0 - anisotropy * ( 1.0 - roughness ) ) ) ) );
					return getIBLRetroRadiance( viewDir, bentNormal, roughness );
				#else
					return vec3( 0.0 );
				#endif
			}
		#endif
	#endif
#endif`,Ig=`ToonMaterial material;
material.diffuseColor = diffuseColor.rgb;`,Dg=`varying vec3 vViewPosition;
struct ToonMaterial {
	vec3 diffuseColor;
};
void RE_Direct_Toon( const in IncidentLight directLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in ToonMaterial material, inout ReflectedLight reflectedLight ) {
	vec3 irradiance = getGradientIrradiance( geometryNormal, directLight.direction ) * directLight.color;
	reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
}
void RE_IndirectDiffuse_Toon( const in vec3 irradiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in ToonMaterial material, inout ReflectedLight reflectedLight ) {
	reflectedLight.indirectDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
}
#define RE_Direct				RE_Direct_Toon
#define RE_IndirectDiffuse		RE_IndirectDiffuse_Toon`,Lg=`BlinnPhongMaterial material;
material.diffuseColor = diffuseColor.rgb;
material.specularColor = specular;
material.specularShininess = shininess;
material.specularStrength = specularStrength;`,Ng=`varying vec3 vViewPosition;
struct BlinnPhongMaterial {
	vec3 diffuseColor;
	vec3 specularColor;
	float specularShininess;
	float specularStrength;
};
void RE_Direct_BlinnPhong( const in IncidentLight directLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in BlinnPhongMaterial material, inout ReflectedLight reflectedLight ) {
	float dotNL = saturate( dot( geometryNormal, directLight.direction ) );
	vec3 irradiance = dotNL * directLight.color;
	reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
	reflectedLight.directSpecular += irradiance * BRDF_BlinnPhong( directLight.direction, geometryViewDir, geometryNormal, material.specularColor, material.specularShininess ) * material.specularStrength;
}
void RE_IndirectDiffuse_BlinnPhong( const in vec3 irradiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in BlinnPhongMaterial material, inout ReflectedLight reflectedLight ) {
	reflectedLight.indirectDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
}
#define RE_Direct				RE_Direct_BlinnPhong
#define RE_IndirectDiffuse		RE_IndirectDiffuse_BlinnPhong`,Ug=`PhysicalMaterial material;
material.diffuseColor = diffuseColor.rgb;
material.diffuseContribution = diffuseColor.rgb * ( 1.0 - metalnessFactor );
material.metalness = metalnessFactor;
vec3 dxy = max( abs( dFdx( nonPerturbedNormal ) ), abs( dFdy( nonPerturbedNormal ) ) );
float geometryRoughness = max( max( dxy.x, dxy.y ), dxy.z );
material.roughness = max( roughnessFactor, 0.045 );
material.roughness += geometryRoughness;
material.roughness = min( material.roughness, 1.0 );
#ifdef USE_DIFFUSE_ROUGHNESS
	float diffuseRoughnessFactor = diffuseRoughness;
	#ifdef USE_DIFFUSE_ROUGHNESSMAP
		diffuseRoughnessFactor *= texture2D( diffuseRoughnessMap, vDiffuseRoughnessMapUv ).r;
	#endif
	material.diffuseRoughness = saturate( diffuseRoughnessFactor );
#endif
#ifdef IOR
	material.ior = ior;
	#ifdef USE_SPECULAR
		float specularIntensityFactor = specularIntensity;
		vec3 specularColorFactor = specularColor;
		#ifdef USE_SPECULAR_COLORMAP
			specularColorFactor *= texture2D( specularColorMap, vSpecularColorMapUv ).rgb;
		#endif
		#ifdef USE_SPECULAR_INTENSITYMAP
			specularIntensityFactor *= texture2D( specularIntensityMap, vSpecularIntensityMapUv ).a;
		#endif
		material.specularF90 = mix( specularIntensityFactor, 1.0, metalnessFactor );
	#else
		float specularIntensityFactor = 1.0;
		vec3 specularColorFactor = vec3( 1.0 );
		material.specularF90 = 1.0;
	#endif
	material.specularColor = min( pow2( ( material.ior - 1.0 ) / ( material.ior + 1.0 ) ) * specularColorFactor, vec3( 1.0 ) ) * specularIntensityFactor;
	material.specularColorBlended = mix( material.specularColor, diffuseColor.rgb, metalnessFactor );
#else
	material.specularColor = vec3( 0.04 );
	material.specularColorBlended = mix( material.specularColor, diffuseColor.rgb, metalnessFactor );
	material.specularF90 = 1.0;
#endif
#ifdef USE_CLEARCOAT
	material.clearcoat = clearcoat;
	material.clearcoatRoughness = clearcoatRoughness;
	material.clearcoatF0 = vec3( 0.04 );
	material.clearcoatF90 = 1.0;
	#ifdef USE_CLEARCOATMAP
		material.clearcoat *= texture2D( clearcoatMap, vClearcoatMapUv ).x;
	#endif
	#ifdef USE_CLEARCOAT_ROUGHNESSMAP
		material.clearcoatRoughness *= texture2D( clearcoatRoughnessMap, vClearcoatRoughnessMapUv ).y;
	#endif
	material.clearcoat = saturate( material.clearcoat );	material.clearcoatRoughness = max( material.clearcoatRoughness, 0.045 );
	material.clearcoatRoughness += geometryRoughness;
	material.clearcoatRoughness = min( material.clearcoatRoughness, 1.0 );
#endif
#ifdef USE_DISPERSION
	material.dispersion = dispersion;
#endif
#ifdef USE_RETROREFLECTION
	material.retroreflectivity = retroreflectivity;
#endif
#ifdef USE_IRIDESCENCE
	material.iridescence = iridescence;
	material.iridescenceIOR = iridescenceIOR;
	#ifdef USE_IRIDESCENCEMAP
		material.iridescence *= texture2D( iridescenceMap, vIridescenceMapUv ).r;
	#endif
	#ifdef USE_IRIDESCENCE_THICKNESSMAP
		material.iridescenceThickness = (iridescenceThicknessMaximum - iridescenceThicknessMinimum) * texture2D( iridescenceThicknessMap, vIridescenceThicknessMapUv ).g + iridescenceThicknessMinimum;
	#else
		material.iridescenceThickness = iridescenceThicknessMaximum;
	#endif
#endif
#ifdef USE_SHEEN
	material.sheenColor = sheenColor;
	#ifdef USE_SHEEN_COLORMAP
		material.sheenColor *= texture2D( sheenColorMap, vSheenColorMapUv ).rgb;
	#endif
	material.sheenRoughness = clamp( sheenRoughness, 0.0001, 1.0 );
	#ifdef USE_SHEEN_ROUGHNESSMAP
		material.sheenRoughness *= texture2D( sheenRoughnessMap, vSheenRoughnessMapUv ).a;
	#endif
#endif
#ifdef USE_ANISOTROPY
	#ifdef USE_ANISOTROPYMAP
		mat2 anisotropyMat = mat2( anisotropyVector.x, anisotropyVector.y, - anisotropyVector.y, anisotropyVector.x );
		vec3 anisotropyPolar = texture2D( anisotropyMap, vAnisotropyMapUv ).rgb;
		vec2 anisotropyV = anisotropyMat * normalize( 2.0 * anisotropyPolar.rg - vec2( 1.0 ) ) * anisotropyPolar.b;
	#else
		vec2 anisotropyV = anisotropyVector;
	#endif
	material.anisotropy = length( anisotropyV );
	if( material.anisotropy == 0.0 ) {
		anisotropyV = vec2( 1.0, 0.0 );
	} else {
		anisotropyV /= material.anisotropy;
		material.anisotropy = saturate( material.anisotropy );
	}
	material.alphaT = mix( pow2( material.roughness ), 1.0, pow2( material.anisotropy ) );
	material.anisotropyT = tbn[ 0 ] * anisotropyV.x + tbn[ 1 ] * anisotropyV.y;
	material.anisotropyB = tbn[ 1 ] * anisotropyV.x - tbn[ 0 ] * anisotropyV.y;
#endif`,Fg=`uniform sampler2D dfgLUT;
struct PhysicalMaterial {
	vec3 diffuseColor;
	vec3 diffuseContribution;
	vec3 specularColor;
	vec3 specularColorBlended;
	float roughness;
	float metalness;
	float specularF90;
	float dispersion;
	vec2 dfg;
	vec3 multiScatteringCompensation;
	#ifdef USE_DIFFUSE_ROUGHNESS
		float diffuseRoughness;
	#endif
	#ifdef USE_RETROREFLECTION
		float retroreflectivity;
	#endif
	#ifdef USE_CLEARCOAT
		float clearcoat;
		float clearcoatRoughness;
		vec3 clearcoatF0;
		float clearcoatF90;
	#endif
	#ifdef USE_IRIDESCENCE
		float iridescence;
		float iridescenceIOR;
		float iridescenceThickness;
		vec3 iridescenceFresnel;
		vec3 iridescenceF0Dielectric;
		vec3 iridescenceF0Metallic;
	#endif
	#ifdef USE_SHEEN
		vec3 sheenColor;
		float sheenRoughness;
	#endif
	#ifdef IOR
		float ior;
	#endif
	#ifdef USE_TRANSMISSION
		float transmission;
		float transmissionAlpha;
		float thickness;
		float attenuationDistance;
		vec3 attenuationColor;
	#endif
	#ifdef USE_ANISOTROPY
		float anisotropy;
		float alphaT;
		vec3 anisotropyT;
		vec3 anisotropyB;
	#endif
};
vec3 clearcoatSpecularDirect = vec3( 0.0 );
vec3 clearcoatSpecularIndirect = vec3( 0.0 );
vec3 sheenSpecularDirect = vec3( 0.0 );
vec3 sheenSpecularIndirect = vec3(0.0 );
vec3 Schlick_to_F0( const in vec3 f, const in float f90, const in float dotVH ) {
    float x = clamp( 1.0 - dotVH, 0.0, 1.0 );
    float x2 = x * x;
    float x5 = clamp( x * x2 * x2, 0.0, 0.9999 );
    return ( f - vec3( f90 ) * x5 ) / ( 1.0 - x5 );
}
#ifdef USE_DIFFUSE_ROUGHNESS
	const float EON_EPSILON = 1e-7;
	float FON_DirectionalAlbedo( const in float mu, const in float roughness, const in float A ) {
		float muComp = 1.0 - mu;
		float gOverPi = muComp * ( 0.0571085289 + muComp * ( 0.491881867 + muComp * ( - 0.332181442 + muComp * 0.0714429953 ) ) );
		return A * ( 1.0 + roughness * gOverPi );
	}
	vec3 BRDF_EON( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, const in vec3 diffuseColor, const in float roughness ) {
		vec3 rho = saturate( diffuseColor );
		if ( roughness <= EON_EPSILON ) return BRDF_Lambert( rho );
		float dotNL = saturate( dot( normal, lightDir ) );
		float dotNV = saturate( dot( normal, viewDir ) );
		float s = dot( lightDir, viewDir ) - dotNL * dotNV;
		float sOverT = ( s > 0.0 ) ? s / max( max( dotNL, dotNV ), EON_EPSILON ) : s;
		float A = 1.0 / ( 1.0 + ( 0.5 - 2.0 / ( 3.0 * PI ) ) * roughness );
		vec3 singleScatter = rho * RECIPROCAL_PI * A * ( 1.0 + roughness * sOverT );
		float averageAlbedo = A * ( 1.0 + ( 2.0 / 3.0 - 28.0 / ( 15.0 * PI ) ) * roughness );
		float albedoV = FON_DirectionalAlbedo( dotNV, roughness, A );
		float albedoL = FON_DirectionalAlbedo( dotNL, roughness, A );
		vec3 rhoMultiScatter = rho * rho * averageAlbedo / max( vec3( EON_EPSILON ), vec3( 1.0 ) - rho * ( 1.0 - averageAlbedo ) );
		vec3 multiScatter = rhoMultiScatter * RECIPROCAL_PI
			* max( EON_EPSILON, 1.0 - albedoV )
			* max( EON_EPSILON, 1.0 - albedoL )
			/ max( EON_EPSILON, 1.0 - averageAlbedo );
		return singleScatter + multiScatter;
	}
	vec3 EON_DirectionalAlbedo( const in vec3 diffuseColor, const in float roughness, const in float dotNV ) {
		vec3 rho = saturate( diffuseColor );
		if ( roughness <= EON_EPSILON ) return rho;
		float A = 1.0 / ( 1.0 + ( 0.5 - 2.0 / ( 3.0 * PI ) ) * roughness );
		float directionalAlbedo = FON_DirectionalAlbedo( dotNV, roughness, A );
		float averageAlbedo = A * ( 1.0 + ( 2.0 / 3.0 - 28.0 / ( 15.0 * PI ) ) * roughness );
		vec3 rhoMultiScatter = rho * rho * averageAlbedo / max( vec3( EON_EPSILON ), vec3( 1.0 ) - rho * ( 1.0 - averageAlbedo ) );
		return rho * directionalAlbedo + rhoMultiScatter * ( 1.0 - directionalAlbedo );
	}
#endif
float V_GGX_SmithCorrelated( const in float alpha, const in float dotNL, const in float dotNV ) {
	float a2 = pow2( alpha );
	float gv = dotNL * sqrt( a2 + ( 1.0 - a2 ) * pow2( dotNV ) );
	float gl = dotNV * sqrt( a2 + ( 1.0 - a2 ) * pow2( dotNL ) );
	return 0.5 / max( gv + gl, EPSILON );
}
float D_GGX( const in float alpha, const in float dotNH ) {
	float a2 = pow2( alpha );
	float denom = pow2( dotNH ) * ( a2 - 1.0 ) + 1.0;
	return RECIPROCAL_PI * a2 / pow2( denom );
}
#ifdef USE_ANISOTROPY
	float V_GGX_SmithCorrelated_Anisotropic( const in float alphaT, const in float alphaB, const in float dotTV, const in float dotBV, const in float dotTL, const in float dotBL, const in float dotNV, const in float dotNL ) {
		float gv = dotNL * length( vec3( alphaT * dotTV, alphaB * dotBV, dotNV ) );
		float gl = dotNV * length( vec3( alphaT * dotTL, alphaB * dotBL, dotNL ) );
		return 0.5 / max( gv + gl, EPSILON );
	}
	float D_GGX_Anisotropic( const in float alphaT, const in float alphaB, const in float dotNH, const in float dotTH, const in float dotBH ) {
		float a2 = alphaT * alphaB;
		highp vec3 v = vec3( alphaB * dotTH, alphaT * dotBH, a2 * dotNH );
		highp float v2 = dot( v, v );
		float w2 = a2 / v2;
		return RECIPROCAL_PI * a2 * pow2 ( w2 );
	}
#endif
#ifdef USE_CLEARCOAT
	vec3 BRDF_GGX_Clearcoat( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, const in PhysicalMaterial material) {
		vec3 f0 = material.clearcoatF0;
		float f90 = material.clearcoatF90;
		float roughness = max( material.clearcoatRoughness, 0.045 );
		float alpha = pow2( roughness );
		vec3 halfDir = normalize( lightDir + viewDir );
		float dotNL = saturate( dot( normal, lightDir ) );
		float dotNV = saturate( dot( normal, viewDir ) );
		float dotNH = saturate( dot( normal, halfDir ) );
		float dotVH = saturate( dot( viewDir, halfDir ) );
		vec3 F = F_Schlick( f0, f90, dotVH );
		float V = V_GGX_SmithCorrelated( alpha, dotNL, dotNV );
		float D = D_GGX( alpha, dotNH );
		return F * ( V * D );
	}
#endif
vec3 BRDF_GGX( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, const in PhysicalMaterial material ) {
	vec3 f0 = material.specularColorBlended;
	float f90 = material.specularF90;
	float roughness = max( material.roughness, 0.045 );
	float alpha = pow2( roughness );
	vec3 halfDir = normalize( lightDir + viewDir );
	float dotNL = saturate( dot( normal, lightDir ) );
	float dotNV = saturate( dot( normal, viewDir ) );
	float dotNH = saturate( dot( normal, halfDir ) );
	float dotVH = saturate( dot( viewDir, halfDir ) );
	vec3 F = F_Schlick( f0, f90, dotVH );
	#ifdef USE_IRIDESCENCE
		F = mix( F, material.iridescenceFresnel, material.iridescence );
	#endif
	#ifdef USE_ANISOTROPY
		float dotTL = dot( material.anisotropyT, lightDir );
		float dotTV = dot( material.anisotropyT, viewDir );
		float dotTH = dot( material.anisotropyT, halfDir );
		float dotBL = dot( material.anisotropyB, lightDir );
		float dotBV = dot( material.anisotropyB, viewDir );
		float dotBH = dot( material.anisotropyB, halfDir );
		float alphaT = max( material.alphaT, alpha );
		float V = V_GGX_SmithCorrelated_Anisotropic( alphaT, alpha, dotTV, dotBV, dotTL, dotBL, dotNV, dotNL );
		float D = D_GGX_Anisotropic( alphaT, alpha, dotNH, dotTH, dotBH );
	#else
		float V = V_GGX_SmithCorrelated( alpha, dotNL, dotNV );
		float D = D_GGX( alpha, dotNH );
	#endif
	return F * ( V * D );
}
vec2 LTC_Uv( const in vec3 N, const in vec3 V, const in float roughness ) {
	const float LUT_SIZE = 64.0;
	const float LUT_SCALE = ( LUT_SIZE - 1.0 ) / LUT_SIZE;
	const float LUT_BIAS = 0.5 / LUT_SIZE;
	float dotNV = saturate( dot( N, V ) );
	vec2 uv = vec2( roughness, sqrt( 1.0 - dotNV ) );
	uv = uv * LUT_SCALE + LUT_BIAS;
	return uv;
}
float LTC_ClippedSphereFormFactor( const in vec3 f ) {
	float l = length( f );
	return max( ( l * l + f.z ) / ( l + 1.0 ), 0.0 );
}
vec3 LTC_EdgeVectorFormFactor( const in vec3 v1, const in vec3 v2 ) {
	float x = dot( v1, v2 );
	float y = abs( x );
	float a = 0.8543985 + ( 0.4965155 + 0.0145206 * y ) * y;
	float b = 3.4175940 + ( 4.1616724 + y ) * y;
	float v = a / b;
	float theta_sintheta = ( x > 0.0 ) ? v : 0.5 * inversesqrt( max( 1.0 - x * x, 1e-7 ) ) - v;
	return cross( v1, v2 ) * theta_sintheta;
}
vec3 LTC_Evaluate( const in vec3 N, const in vec3 V, const in vec3 P, const in mat3 mInv, const in vec3 rectCoords[ 4 ] ) {
	vec3 v1 = rectCoords[ 1 ] - rectCoords[ 0 ];
	vec3 v2 = rectCoords[ 3 ] - rectCoords[ 0 ];
	vec3 lightNormal = cross( v1, v2 );
	if( dot( lightNormal, P - rectCoords[ 0 ] ) < 0.0 ) return vec3( 0.0 );
	vec3 T1, T2;
	T1 = normalize( V - N * dot( V, N ) );
	T2 = - cross( N, T1 );
	mat3 mat = mInv * transpose( mat3( T1, T2, N ) );
	vec3 coords[ 4 ];
	coords[ 0 ] = mat * ( rectCoords[ 0 ] - P );
	coords[ 1 ] = mat * ( rectCoords[ 1 ] - P );
	coords[ 2 ] = mat * ( rectCoords[ 2 ] - P );
	coords[ 3 ] = mat * ( rectCoords[ 3 ] - P );
	coords[ 0 ] = normalize( coords[ 0 ] );
	coords[ 1 ] = normalize( coords[ 1 ] );
	coords[ 2 ] = normalize( coords[ 2 ] );
	coords[ 3 ] = normalize( coords[ 3 ] );
	vec3 vectorFormFactor = vec3( 0.0 );
	vectorFormFactor += LTC_EdgeVectorFormFactor( coords[ 0 ], coords[ 1 ] );
	vectorFormFactor += LTC_EdgeVectorFormFactor( coords[ 1 ], coords[ 2 ] );
	vectorFormFactor += LTC_EdgeVectorFormFactor( coords[ 2 ], coords[ 3 ] );
	vectorFormFactor += LTC_EdgeVectorFormFactor( coords[ 3 ], coords[ 0 ] );
	float result = LTC_ClippedSphereFormFactor( vectorFormFactor );
	return vec3( result );
}
#if defined( USE_SHEEN )
float D_Charlie( float roughness, float dotNH ) {
	float alpha = pow2( roughness );
	float invAlpha = 1.0 / alpha;
	float cos2h = dotNH * dotNH;
	float sin2h = max( 1.0 - cos2h, 0.0078125 );
	return ( 2.0 + invAlpha ) * pow( sin2h, invAlpha * 0.5 ) / ( 2.0 * PI );
}
float V_Neubelt( float dotNV, float dotNL ) {
	return saturate( 1.0 / ( 4.0 * ( dotNL + dotNV - dotNL * dotNV ) ) );
}
vec3 BRDF_Sheen( const in vec3 lightDir, const in vec3 viewDir, const in vec3 normal, vec3 sheenColor, const in float sheenRoughness ) {
	vec3 halfDir = normalize( lightDir + viewDir );
	float dotNL = saturate( dot( normal, lightDir ) );
	float dotNV = saturate( dot( normal, viewDir ) );
	float dotNH = saturate( dot( normal, halfDir ) );
	float D = D_Charlie( sheenRoughness, dotNH );
	float V = V_Neubelt( dotNV, dotNL );
	return sheenColor * ( D * V );
}
#endif
float IBLSheenBRDF( const in vec3 normal, const in vec3 viewDir, const in float roughness ) {
	float dotNV = saturate( dot( normal, viewDir ) );
	float r2 = roughness * roughness;
	float rInv = 1.0 / ( roughness + 0.1 );
	float a = -1.9362 + 1.0678 * roughness + 0.4573 * r2 - 0.8469 * rInv;
	float b = -0.6014 + 0.5538 * roughness - 0.4670 * r2 - 0.1255 * rInv;
	float DG = exp( a * dotNV + b );
	return saturate( DG );
}
vec3 EnvironmentBRDF( const in vec3 normal, const in vec3 viewDir, const in vec3 specularColor, const in float specularF90, const in float roughness ) {
	float dotNV = saturate( dot( normal, viewDir ) );
	vec2 fab = texture2D( dfgLUT, vec2( roughness, dotNV ) ).rg;
	return specularColor * fab.x + specularF90 * fab.y;
}
#ifdef USE_IRIDESCENCE
void computeMultiscatteringIridescence( const in vec2 fab, const in vec3 specularColor, const in float specularF90, const in float iridescence, const in vec3 iridescenceF0, inout vec3 singleScatter, inout vec3 multiScatter ) {
#else
void computeMultiscattering( const in vec2 fab, const in vec3 specularColor, const in float specularF90, inout vec3 singleScatter, inout vec3 multiScatter ) {
#endif
	#ifdef USE_IRIDESCENCE
		vec3 Fr = mix( specularColor, iridescenceF0, iridescence );
	#else
		vec3 Fr = specularColor;
	#endif
	vec3 FssEss = Fr * fab.x + specularF90 * fab.y;
	float Ess = fab.x + fab.y;
	float Ems = 1.0 - Ess;
	vec3 Favg = Fr + ( 1.0 - Fr ) * 0.047619;	vec3 Fms = FssEss * Favg / ( 1.0 - Ems * Favg );
	singleScatter += FssEss;
	multiScatter += Fms * Ems;
}
#if NUM_RECT_AREA_LIGHTS > 0
	void RE_Direct_RectArea_Physical( const in RectAreaLight rectAreaLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in PhysicalMaterial material, inout ReflectedLight reflectedLight ) {
		vec3 normal = geometryNormal;
		vec3 viewDir = geometryViewDir;
		vec3 position = geometryPosition;
		vec3 lightPos = rectAreaLight.position;
		vec3 halfWidth = rectAreaLight.halfWidth;
		vec3 halfHeight = rectAreaLight.halfHeight;
		vec3 lightColor = rectAreaLight.color;
		float roughness = material.roughness;
		vec3 rectCoords[ 4 ];
		rectCoords[ 0 ] = lightPos + halfWidth - halfHeight;		rectCoords[ 1 ] = lightPos - halfWidth - halfHeight;
		rectCoords[ 2 ] = lightPos - halfWidth + halfHeight;
		rectCoords[ 3 ] = lightPos + halfWidth + halfHeight;
		vec2 uv = LTC_Uv( normal, viewDir, roughness );
		vec4 t1 = texture2D( ltc_1, uv );
		vec4 t2 = texture2D( ltc_2, uv );
		mat3 mInv = mat3(
			vec3( t1.x, 0, t1.y ),
			vec3(    0, 1,    0 ),
			vec3( t1.z, 0, t1.w )
		);
		vec3 fresnel = ( material.specularColorBlended * t2.x + ( material.specularF90 - material.specularColorBlended ) * t2.y );
		reflectedLight.directSpecular += lightColor * fresnel * LTC_Evaluate( normal, viewDir, position, mInv, rectCoords );
		reflectedLight.directDiffuse += lightColor * material.diffuseContribution * LTC_Evaluate( normal, viewDir, position, mat3( 1.0 ), rectCoords );
		#ifdef USE_CLEARCOAT
			vec3 Ncc = geometryClearcoatNormal;
			vec2 uvClearcoat = LTC_Uv( Ncc, viewDir, material.clearcoatRoughness );
			vec4 t1Clearcoat = texture2D( ltc_1, uvClearcoat );
			vec4 t2Clearcoat = texture2D( ltc_2, uvClearcoat );
			mat3 mInvClearcoat = mat3(
				vec3( t1Clearcoat.x, 0, t1Clearcoat.y ),
				vec3(             0, 1,             0 ),
				vec3( t1Clearcoat.z, 0, t1Clearcoat.w )
			);
			vec3 fresnelClearcoat = material.clearcoatF0 * t2Clearcoat.x + ( material.clearcoatF90 - material.clearcoatF0 ) * t2Clearcoat.y;
			clearcoatSpecularDirect += lightColor * fresnelClearcoat * LTC_Evaluate( Ncc, viewDir, position, mInvClearcoat, rectCoords );
		#endif
	}
#endif
void RE_Direct_Physical( const in IncidentLight directLight, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in PhysicalMaterial material, inout ReflectedLight reflectedLight ) {
	float dotNL = saturate( dot( geometryNormal, directLight.direction ) );
	vec3 irradiance = dotNL * directLight.color;
	#ifdef USE_CLEARCOAT
		float dotNLcc = saturate( dot( geometryClearcoatNormal, directLight.direction ) );
		vec3 ccIrradiance = dotNLcc * directLight.color;
		clearcoatSpecularDirect += ccIrradiance * BRDF_GGX_Clearcoat( directLight.direction, geometryViewDir, geometryClearcoatNormal, material );
	#endif
	#ifdef USE_SHEEN
 
 		sheenSpecularDirect += irradiance * BRDF_Sheen( directLight.direction, geometryViewDir, geometryNormal, material.sheenColor, material.sheenRoughness );
 
 		float sheenAlbedoV = IBLSheenBRDF( geometryNormal, geometryViewDir, material.sheenRoughness );
 		float sheenAlbedoL = IBLSheenBRDF( geometryNormal, directLight.direction, material.sheenRoughness );
 
 		float sheenEnergyComp = 1.0 - max3( material.sheenColor ) * max( sheenAlbedoV, sheenAlbedoL );
 
 		irradiance *= sheenEnergyComp;
 
 	#endif
	vec3 specularBRDF = BRDF_GGX( directLight.direction, geometryViewDir, geometryNormal, material );
	#ifdef USE_RETROREFLECTION
		vec3 retroViewDir = reflect( - geometryViewDir, geometryNormal );
		vec3 retroSpecularBRDF = BRDF_GGX( directLight.direction, retroViewDir, geometryNormal, material );
		specularBRDF = mix( specularBRDF, retroSpecularBRDF, saturate( material.retroreflectivity ) );
	#endif
	reflectedLight.directSpecular += irradiance * specularBRDF * material.multiScatteringCompensation;
	vec3 halfDir = normalize( directLight.direction + geometryViewDir );
	float dotVH = saturate( dot( geometryViewDir, halfDir ) );
	vec3 F = F_Schlick( material.specularColor, material.specularF90, dotVH );
	#ifdef USE_RETROREFLECTION
		vec3 retroHalfDir = normalize( directLight.direction + retroViewDir );
		float dotRetroVH = saturate( dot( retroViewDir, retroHalfDir ) );
		vec3 retroF = F_Schlick( material.specularColor, material.specularF90, dotRetroVH );
		F = mix( F, retroF, saturate( material.retroreflectivity ) );
	#endif
	#ifdef USE_DIFFUSE_ROUGHNESS
		vec3 diffuseBRDF = BRDF_EON( directLight.direction, geometryViewDir, geometryNormal, material.diffuseColor, material.diffuseRoughness ) * ( 1.0 - material.metalness );
	#else
		vec3 diffuseBRDF = BRDF_Lambert( material.diffuseContribution );
	#endif
	reflectedLight.directDiffuse += irradiance * diffuseBRDF * ( 1.0 - F );
}
void RE_IndirectDiffuse_Physical( const in vec3 irradiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in PhysicalMaterial material, inout ReflectedLight reflectedLight ) {
	vec3 singleScattering = vec3( 0.0 );
	vec3 multiScattering = vec3( 0.0 );
	#ifdef USE_IRIDESCENCE
		computeMultiscatteringIridescence( material.dfg, material.specularColor, material.specularF90, material.iridescence, material.iridescenceF0Dielectric, singleScattering, multiScattering );
	#else
		computeMultiscattering( material.dfg, material.specularColor, material.specularF90, singleScattering, multiScattering );
	#endif
	#ifdef USE_DIFFUSE_ROUGHNESS
		float dotNV = saturate( dot( geometryNormal, geometryViewDir ) );
		vec3 diffuseAlbedo = EON_DirectionalAlbedo( material.diffuseColor, material.diffuseRoughness, dotNV ) * ( 1.0 - material.metalness );
		vec3 diffuse = irradiance * RECIPROCAL_PI * diffuseAlbedo * ( 1.0 - singleScattering - multiScattering );
	#else
		vec3 diffuse = irradiance * BRDF_Lambert( material.diffuseContribution ) * ( 1.0 - singleScattering - multiScattering );
	#endif
	#ifdef USE_SHEEN
		float sheenAlbedo = IBLSheenBRDF( geometryNormal, geometryViewDir, material.sheenRoughness );
		sheenSpecularIndirect += irradiance * material.sheenColor * sheenAlbedo * RECIPROCAL_PI;
		float sheenEnergyComp = 1.0 - max3( material.sheenColor ) * sheenAlbedo;
		diffuse *= sheenEnergyComp;
	#endif
	reflectedLight.indirectDiffuse += diffuse;
}
void RE_IndirectSpecular_Physical( const in vec3 radiance, const in vec3 irradiance, const in vec3 clearcoatRadiance, const in vec3 geometryPosition, const in vec3 geometryNormal, const in vec3 geometryViewDir, const in vec3 geometryClearcoatNormal, const in PhysicalMaterial material, inout ReflectedLight reflectedLight) {
	#ifdef USE_CLEARCOAT
		clearcoatSpecularIndirect += clearcoatRadiance * EnvironmentBRDF( geometryClearcoatNormal, geometryViewDir, material.clearcoatF0, material.clearcoatF90, material.clearcoatRoughness );
	#endif
	#ifdef USE_SHEEN
		sheenSpecularIndirect += irradiance * material.sheenColor * IBLSheenBRDF( geometryNormal, geometryViewDir, material.sheenRoughness ) * RECIPROCAL_PI;
 	#endif
	vec3 singleScatteringDielectric = vec3( 0.0 );
	vec3 multiScatteringDielectric = vec3( 0.0 );
	vec3 singleScatteringMetallic = vec3( 0.0 );
	vec3 multiScatteringMetallic = vec3( 0.0 );
	#ifdef USE_IRIDESCENCE
		computeMultiscatteringIridescence( material.dfg, material.specularColor, material.specularF90, material.iridescence, material.iridescenceF0Dielectric, singleScatteringDielectric, multiScatteringDielectric );
		computeMultiscatteringIridescence( material.dfg, material.diffuseColor, material.specularF90, material.iridescence, material.iridescenceF0Metallic, singleScatteringMetallic, multiScatteringMetallic );
	#else
		computeMultiscattering( material.dfg, material.specularColor, material.specularF90, singleScatteringDielectric, multiScatteringDielectric );
		computeMultiscattering( material.dfg, material.diffuseColor, material.specularF90, singleScatteringMetallic, multiScatteringMetallic );
	#endif
	vec3 singleScattering = mix( singleScatteringDielectric, singleScatteringMetallic, material.metalness );
	vec3 multiScattering = mix( multiScatteringDielectric, multiScatteringMetallic, material.metalness );
	vec3 totalScatteringDielectric = singleScatteringDielectric + multiScatteringDielectric;
	#ifdef USE_DIFFUSE_ROUGHNESS
		float dotNV = saturate( dot( geometryNormal, geometryViewDir ) );
		vec3 diffuse = EON_DirectionalAlbedo( material.diffuseColor, material.diffuseRoughness, dotNV ) * ( 1.0 - material.metalness ) * ( 1.0 - totalScatteringDielectric );
	#else
		vec3 diffuse = material.diffuseContribution * ( 1.0 - totalScatteringDielectric );
	#endif
	vec3 cosineWeightedIrradiance = irradiance * RECIPROCAL_PI;
	vec3 indirectSpecular = radiance * singleScattering;
	indirectSpecular += multiScattering * cosineWeightedIrradiance;
	vec3 indirectDiffuse = diffuse * cosineWeightedIrradiance;
	#ifdef USE_SHEEN
		float sheenAlbedo = IBLSheenBRDF( geometryNormal, geometryViewDir, material.sheenRoughness );
		float sheenEnergyComp = 1.0 - max3( material.sheenColor ) * sheenAlbedo;
		indirectSpecular *= sheenEnergyComp;
		indirectDiffuse *= sheenEnergyComp;
	#endif
	reflectedLight.indirectSpecular += indirectSpecular;
	reflectedLight.indirectDiffuse += indirectDiffuse;
}
#define RE_Direct				RE_Direct_Physical
#define RE_Direct_RectArea		RE_Direct_RectArea_Physical
#define RE_IndirectDiffuse		RE_IndirectDiffuse_Physical
#define RE_IndirectSpecular		RE_IndirectSpecular_Physical
float computeSpecularOcclusion( const in float dotNV, const in float ambientOcclusion, const in float roughness ) {
	return saturate( pow( dotNV + ambientOcclusion, exp2( - 16.0 * roughness - 1.0 ) ) - 1.0 + ambientOcclusion );
}`,Og=`
vec3 geometryPosition = - vViewPosition;
vec3 geometryNormal = normal;
vec3 geometryViewDir = ( isOrthographic ) ? vec3( 0, 0, 1 ) : normalize( vViewPosition );
vec3 geometryClearcoatNormal = vec3( 0.0 );
#ifdef USE_CLEARCOAT
	geometryClearcoatNormal = clearcoatNormal;
#endif
#ifdef USE_IRIDESCENCE
	float dotNVi = saturate( dot( normal, geometryViewDir ) );
	if ( material.iridescenceThickness == 0.0 ) {
		material.iridescence = 0.0;
	} else {
		material.iridescence = saturate( material.iridescence );
	}
	if ( material.iridescence > 0.0 ) {
		vec3 iridescenceFresnelDielectric = evalIridescence( 1.0, material.iridescenceIOR, dotNVi, material.iridescenceThickness, material.specularColor );
		vec3 iridescenceFresnelMetallic = evalIridescence( 1.0, material.iridescenceIOR, dotNVi, material.iridescenceThickness, material.diffuseColor );
		material.iridescenceFresnel = mix( iridescenceFresnelDielectric, iridescenceFresnelMetallic, material.metalness );
		material.iridescenceF0Dielectric = Schlick_to_F0( iridescenceFresnelDielectric, 1.0, dotNVi );
		material.iridescenceF0Metallic = Schlick_to_F0( iridescenceFresnelMetallic, 1.0, dotNVi );
	}
#endif
#ifdef STANDARD
	float dotNVms = saturate( dot( geometryNormal, geometryViewDir ) );
	material.dfg = texture2D( dfgLUT, vec2( material.roughness, dotNVms ) ).rg;
	#if ( NUM_SUN_LIGHTS > 0 || NUM_DIR_LIGHTS > 0 || NUM_POINT_LIGHTS > 0 || NUM_SPOT_LIGHTS > 0 )
		float EssMs = material.dfg.x + material.dfg.y;
		material.multiScatteringCompensation = 1.0 + material.specularColorBlended * ( 1.0 / EssMs - 1.0 );
	#endif
#endif
IncidentLight directLight;
#if ( NUM_POINT_LIGHTS > 0 ) && defined( RE_Direct )
	PointLight pointLight;
	#if defined( USE_SHADOWMAP ) && NUM_POINT_LIGHT_SHADOWS > 0
	PointLightShadow pointLightShadow;
	#endif
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_POINT_LIGHTS; i ++ ) {
		pointLight = pointLights[ i ];
		getPointLightInfo( pointLight, geometryPosition, directLight );
		#if defined( USE_SHADOWMAP ) && ( UNROLLED_LOOP_INDEX < NUM_POINT_LIGHT_SHADOWS ) && ( defined( SHADOWMAP_TYPE_PCF ) || defined( SHADOWMAP_TYPE_BASIC ) )
		pointLightShadow = pointLightShadows[ i ];
		directLight.color *= ( directLight.visible && receiveShadow ) ? getPointShadow( pointShadowMap[ i ], pointLightShadow.shadowMapSize, pointLightShadow.shadowIntensity, pointLightShadow.shadowBias, pointLightShadow.shadowRadius, vPointShadowCoord[ i ], pointLightShadow.shadowCameraNear, pointLightShadow.shadowCameraFar ) : 1.0;
		#endif
		RE_Direct( directLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
	}
	#pragma unroll_loop_end
#endif
#if ( NUM_SPOT_LIGHTS > 0 ) && defined( RE_Direct )
	SpotLight spotLight;
	vec4 spotColor;
	vec3 spotLightCoord;
	bool inSpotLightMap;
	#if defined( USE_SHADOWMAP ) && NUM_SPOT_LIGHT_SHADOWS > 0
	SpotLightShadow spotLightShadow;
	#endif
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_SPOT_LIGHTS; i ++ ) {
		spotLight = spotLights[ i ];
		getSpotLightInfo( spotLight, geometryPosition, directLight );
		#if ( UNROLLED_LOOP_INDEX < NUM_SPOT_LIGHT_SHADOWS_WITH_MAPS )
		#define SPOT_LIGHT_MAP_INDEX UNROLLED_LOOP_INDEX
		#elif ( UNROLLED_LOOP_INDEX < NUM_SPOT_LIGHT_SHADOWS )
		#define SPOT_LIGHT_MAP_INDEX NUM_SPOT_LIGHT_MAPS
		#else
		#define SPOT_LIGHT_MAP_INDEX ( UNROLLED_LOOP_INDEX - NUM_SPOT_LIGHT_SHADOWS + NUM_SPOT_LIGHT_SHADOWS_WITH_MAPS )
		#endif
		#if ( SPOT_LIGHT_MAP_INDEX < NUM_SPOT_LIGHT_MAPS )
			spotLightCoord = vSpotLightCoord[ i ].xyz / vSpotLightCoord[ i ].w;
			inSpotLightMap = all( lessThan( abs( spotLightCoord * 2. - 1. ), vec3( 1.0 ) ) );
			spotColor = texture2D( spotLightMap[ SPOT_LIGHT_MAP_INDEX ], spotLightCoord.xy );
			directLight.color = inSpotLightMap ? directLight.color * spotColor.rgb : directLight.color;
		#endif
		#undef SPOT_LIGHT_MAP_INDEX
		#if defined( USE_SHADOWMAP ) && ( UNROLLED_LOOP_INDEX < NUM_SPOT_LIGHT_SHADOWS )
		spotLightShadow = spotLightShadows[ i ];
		directLight.color *= ( directLight.visible && receiveShadow ) ? getShadow( spotShadowMap[ i ], spotLightShadow.shadowMapSize, spotLightShadow.shadowIntensity, spotLightShadow.shadowBias, spotLightShadow.shadowRadius, vSpotLightCoord[ i ] ) : 1.0;
		#endif
		RE_Direct( directLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
	}
	#pragma unroll_loop_end
#endif
#if ( NUM_SUN_LIGHTS > 0 ) && defined( RE_Direct )
	SunLight sunLight;
	#if defined( USE_SHADOWMAP ) && NUM_SUN_LIGHT_SHADOWS > 0
	SunLightShadow sunLightShadow;
	#endif
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_SUN_LIGHTS; i ++ ) {
		sunLight = sunLights[ i ];
		getSunLightInfo( sunLight, directLight );
		#if defined( USE_SHADOWMAP ) && ( UNROLLED_LOOP_INDEX < NUM_SUN_LIGHT_SHADOWS )
		sunLightShadow = sunLightShadows[ i ];
		directLight.color *= ( directLight.visible && receiveShadow ) ? getSunShadow( sunShadowMap[ i ], sunLightShadow, UNROLLED_LOOP_INDEX ) : 1.0;
		#endif
		RE_Direct( directLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
	}
	#pragma unroll_loop_end
#endif
#if ( NUM_DIR_LIGHTS > 0 ) && defined( RE_Direct )
	DirectionalLight directionalLight;
	#if defined( USE_SHADOWMAP ) && NUM_DIR_LIGHT_SHADOWS > 0
	DirectionalLightShadow directionalLightShadow;
	#endif
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_DIR_LIGHTS; i ++ ) {
		directionalLight = directionalLights[ i ];
		getDirectionalLightInfo( directionalLight, directLight );
		#if defined( USE_SHADOWMAP ) && ( UNROLLED_LOOP_INDEX < NUM_DIR_LIGHT_SHADOWS )
		directionalLightShadow = directionalLightShadows[ i ];
		directLight.color *= ( directLight.visible && receiveShadow ) ? getShadow( directionalShadowMap[ i ], directionalLightShadow.shadowMapSize, directionalLightShadow.shadowIntensity, directionalLightShadow.shadowBias, directionalLightShadow.shadowRadius, vDirectionalShadowCoord[ i ] ) : 1.0;
		#endif
		RE_Direct( directLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
	}
	#pragma unroll_loop_end
#endif
#if ( NUM_RECT_AREA_LIGHTS > 0 ) && defined( RE_Direct_RectArea )
	RectAreaLight rectAreaLight;
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_RECT_AREA_LIGHTS; i ++ ) {
		rectAreaLight = rectAreaLights[ i ];
		RE_Direct_RectArea( rectAreaLight, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
	}
	#pragma unroll_loop_end
#endif
#if defined( RE_IndirectDiffuse )
	vec3 iblIrradiance = vec3( 0.0 );
	vec3 irradiance = getAmbientLightIrradiance( ambientLightColor );
	#if defined( USE_LIGHT_PROBES )
		irradiance += getLightProbeIrradiance( lightProbe, geometryNormal );
	#endif
	#if ( NUM_HEMI_LIGHTS > 0 )
		#pragma unroll_loop_start
		for ( int i = 0; i < NUM_HEMI_LIGHTS; i ++ ) {
			irradiance += getHemisphereLightIrradiance( hemisphereLights[ i ], geometryNormal );
		}
		#pragma unroll_loop_end
	#endif
	#if NUM_LIGHT_PROBE_GRIDS > 0
		vec3 probeWorldPos = ( ( vec4( geometryPosition, 1.0 ) - viewMatrix[ 3 ] ) * viewMatrix ).xyz;
		vec3 probeWorldNormal = transformNormalByInverseViewMatrix( geometryNormal, viewMatrix );
		int probeGrid = - 1;
		#pragma unroll_loop_start
		for ( int i = 0; i < NUM_LIGHT_PROBE_GRIDS; i ++ ) {
			if ( isInsideLightProbeGrid( probeWorldPos, probeWorldNormal, probesMin[ i ], probesMax[ i ], probesResolution[ i ] ) ) probeGrid = UNROLLED_LOOP_INDEX;
		}
		#pragma unroll_loop_end
		#pragma unroll_loop_start
		for ( int i = 0; i < NUM_LIGHT_PROBE_GRIDS; i ++ ) {
			if ( probeGrid == UNROLLED_LOOP_INDEX ) irradiance += getLightProbeGridIrradiance( probesSH[ i ], probesMin[ i ], probesMax[ i ], probesResolution[ i ], probeWorldPos, probeWorldNormal );
		}
		#pragma unroll_loop_end
	#endif
#endif
#if defined( RE_IndirectSpecular )
	vec3 radiance = vec3( 0.0 );
	vec3 clearcoatRadiance = vec3( 0.0 );
#endif`,Bg=`#if defined( RE_IndirectDiffuse )
	#ifdef USE_LIGHTMAP
		vec4 lightMapTexel = texture2D( lightMap, vLightMapUv );
		vec3 lightMapIrradiance = lightMapTexel.rgb * lightMapIntensity;
		irradiance += lightMapIrradiance;
	#endif
	#if defined( USE_ENVMAP ) && defined( ENVMAP_TYPE_PMREM )
		#if defined( STANDARD ) || defined( LAMBERT ) || defined( PHONG )
			iblIrradiance += getIBLIrradiance( geometryNormal );
		#endif
	#endif
#endif
#if defined( USE_ENVMAP ) && defined( RE_IndirectSpecular )
	#ifdef USE_ANISOTROPY
		vec3 iblRadiance = getIBLAnisotropyRadiance( geometryViewDir, geometryNormal, material.roughness, material.anisotropyB, material.anisotropy );
	#else
		vec3 iblRadiance = getIBLRadiance( geometryViewDir, geometryNormal, material.roughness );
	#endif
	#ifdef USE_RETROREFLECTION
		#ifdef USE_ANISOTROPY
			vec3 retroIBLRadiance = getIBLAnisotropyRetroRadiance( geometryViewDir, geometryNormal, material.roughness, material.anisotropyB, material.anisotropy );
		#else
			vec3 retroIBLRadiance = getIBLRetroRadiance( geometryViewDir, geometryNormal, material.roughness );
		#endif
		iblRadiance = mix( iblRadiance, retroIBLRadiance, saturate( material.retroreflectivity ) );
	#endif
	radiance += iblRadiance;
	#ifdef USE_CLEARCOAT
		clearcoatRadiance += getIBLRadiance( geometryViewDir, geometryClearcoatNormal, material.clearcoatRoughness );
	#endif
#endif`,zg=`#if defined( RE_IndirectDiffuse )
	#if defined( LAMBERT ) || defined( PHONG )
		irradiance += iblIrradiance;
	#endif
	RE_IndirectDiffuse( irradiance, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
#endif
#if defined( RE_IndirectSpecular )
	RE_IndirectSpecular( radiance, iblIrradiance, clearcoatRadiance, geometryPosition, geometryNormal, geometryViewDir, geometryClearcoatNormal, material, reflectedLight );
#endif`,kg=`#if NUM_LIGHT_PROBE_GRIDS > 0
uniform highp sampler3D probesSH[ NUM_LIGHT_PROBE_GRIDS ];
uniform vec3 probesMin[ NUM_LIGHT_PROBE_GRIDS ];
uniform vec3 probesMax[ NUM_LIGHT_PROBE_GRIDS ];
uniform vec3 probesResolution[ NUM_LIGHT_PROBE_GRIDS ];
bool isInsideLightProbeGrid( vec3 worldPos, vec3 worldNormal, vec3 probesMin, vec3 probesMax, vec3 probesResolution ) {
	vec3 spacing = ( probesMax - probesMin ) / probesResolution;
	vec3 samplePos = worldPos + worldNormal * spacing * 0.5;
	vec3 margin = spacing * 0.01;
	return all( greaterThanEqual( samplePos, probesMin - margin ) ) && all( lessThanEqual( samplePos, probesMax + margin ) );
}
vec3 getLightProbeGridIrradiance( highp sampler3D probesSH, vec3 probesMin, vec3 probesMax, vec3 probesResolution, vec3 worldPos, vec3 worldNormal ) {
	vec3 res = probesResolution;
	vec3 gridRange = probesMax - probesMin;
	vec3 probeSpacing = gridRange / res;
	vec3 samplePos = worldPos + worldNormal * probeSpacing * 0.5;
	vec3 uvw = clamp( ( samplePos - probesMin ) / gridRange, 0.5 / res, 1.0 - 0.5 / res );
	float nz          = res.z;
	float paddedSlices = nz + 2.0;
	float atlasDepth  = 7.0 * paddedSlices;
	float uvZBase     = uvw.z * nz + 1.0;
	vec4 s0 = texture( probesSH, vec3( uvw.xy, ( uvZBase                       ) / atlasDepth ) );
	vec4 s1 = texture( probesSH, vec3( uvw.xy, ( uvZBase +       paddedSlices   ) / atlasDepth ) );
	vec4 s2 = texture( probesSH, vec3( uvw.xy, ( uvZBase + 2.0 * paddedSlices   ) / atlasDepth ) );
	vec4 s3 = texture( probesSH, vec3( uvw.xy, ( uvZBase + 3.0 * paddedSlices   ) / atlasDepth ) );
	vec4 s4 = texture( probesSH, vec3( uvw.xy, ( uvZBase + 4.0 * paddedSlices   ) / atlasDepth ) );
	vec4 s5 = texture( probesSH, vec3( uvw.xy, ( uvZBase + 5.0 * paddedSlices   ) / atlasDepth ) );
	vec4 s6 = texture( probesSH, vec3( uvw.xy, ( uvZBase + 6.0 * paddedSlices   ) / atlasDepth ) );
	vec3 c0 = s0.xyz;
	vec3 c1 = vec3( s0.w, s1.xy );
	vec3 c2 = vec3( s1.zw, s2.x );
	vec3 c3 = s2.yzw;
	vec3 c4 = s3.xyz;
	vec3 c5 = vec3( s3.w, s4.xy );
	vec3 c6 = vec3( s4.zw, s5.x );
	vec3 c7 = s5.yzw;
	vec3 c8 = s6.xyz;
	float x = worldNormal.x, y = worldNormal.y, z = worldNormal.z;
	vec3 result = c0 * 0.886227;
	result += c1 * 2.0 * 0.511664 * y;
	result += c2 * 2.0 * 0.511664 * z;
	result += c3 * 2.0 * 0.511664 * x;
	result += c4 * 2.0 * 0.429043 * x * y;
	result += c5 * 2.0 * 0.429043 * y * z;
	result += c6 * ( 0.743125 * z * z - 0.247708 );
	result += c7 * 2.0 * 0.429043 * x * z;
	result += c8 * 0.429043 * ( x * x - y * y );
	return max( result, vec3( 0.0 ) );
}
#endif`,Vg=`#if defined( USE_LOGARITHMIC_DEPTH_BUFFER )
	gl_FragDepth = vIsPerspective == 0.0 ? gl_FragCoord.z : log2( vFragDepth ) * logDepthBufFC * 0.5;
#endif`,Gg=`#if defined( USE_LOGARITHMIC_DEPTH_BUFFER )
	uniform float logDepthBufFC;
	varying float vFragDepth;
	varying float vIsPerspective;
#endif`,Hg=`#ifdef USE_LOGARITHMIC_DEPTH_BUFFER
	varying float vFragDepth;
	varying float vIsPerspective;
#endif`,Wg=`#ifdef USE_LOGARITHMIC_DEPTH_BUFFER
	vFragDepth = 1.0 + gl_Position.w;
	vIsPerspective = float( isPerspectiveMatrix( projectionMatrix ) );
#endif`,Xg=`#ifdef USE_MAP
	vec4 sampledDiffuseColor = texture2D( map, vMapUv );
	#ifdef DECODE_VIDEO_TEXTURE
		sampledDiffuseColor = sRGBTransferEOTF( sampledDiffuseColor );
	#endif
	diffuseColor *= sampledDiffuseColor;
#endif`,qg=`#ifdef USE_MAP
	uniform sampler2D map;
#endif`,Yg=`#if defined( USE_MAP ) || defined( USE_ALPHAMAP )
	#if defined( USE_POINTS_UV )
		vec2 uv = vUv;
	#else
		vec2 uv = ( uvTransform * vec3( gl_PointCoord.x, 1.0 - gl_PointCoord.y, 1 ) ).xy;
	#endif
#endif
#ifdef USE_MAP
	diffuseColor *= texture2D( map, uv );
#endif
#ifdef USE_ALPHAMAP
	diffuseColor.a *= texture2D( alphaMap, uv ).g;
#endif`,Zg=`#if defined( USE_POINTS_UV )
	varying vec2 vUv;
#else
	#if defined( USE_MAP ) || defined( USE_ALPHAMAP )
		uniform mat3 uvTransform;
	#endif
#endif
#ifdef USE_MAP
	uniform sampler2D map;
#endif
#ifdef USE_ALPHAMAP
	uniform sampler2D alphaMap;
#endif`,Jg=`float metalnessFactor = metalness;
#ifdef USE_METALNESSMAP
	vec4 texelMetalness = texture2D( metalnessMap, vMetalnessMapUv );
	metalnessFactor *= texelMetalness.b;
#endif`,$g=`#ifdef USE_METALNESSMAP
	uniform sampler2D metalnessMap;
#endif`,Kg=`#ifdef USE_INSTANCING_MORPH
	float morphTargetInfluences[ MORPHTARGETS_COUNT ];
	float morphTargetBaseInfluence = texelFetch( morphTexture, ivec2( 0, gl_InstanceID ), 0 ).r;
	for ( int i = 0; i < MORPHTARGETS_COUNT; i ++ ) {
		morphTargetInfluences[i] =  texelFetch( morphTexture, ivec2( i + 1, gl_InstanceID ), 0 ).r;
	}
#endif`,jg=`#if defined( USE_MORPHCOLORS )
	#if defined( USE_COLOR_ALPHA )
		vColor *= morphTargetBaseInfluence;
	#elif defined( USE_COLOR )
		vColor.rgb *= morphTargetBaseInfluence;
	#endif
	for ( int i = 0; i < MORPHTARGETS_COUNT; i ++ ) {
		#if defined( USE_COLOR_ALPHA )
			if ( morphTargetInfluences[ i ] != 0.0 ) vColor += getMorph( gl_VertexID, i, 2 ) * morphTargetInfluences[ i ];
		#elif defined( USE_COLOR )
			if ( morphTargetInfluences[ i ] != 0.0 ) vColor.rgb += getMorph( gl_VertexID, i, 2 ).rgb * morphTargetInfluences[ i ];
		#endif
	}
#endif`,Qg=`#ifdef USE_MORPHNORMALS
	objectNormal *= morphTargetBaseInfluence;
	for ( int i = 0; i < MORPHTARGETS_COUNT; i ++ ) {
		if ( morphTargetInfluences[ i ] != 0.0 ) objectNormal += getMorph( gl_VertexID, i, 1 ).xyz * morphTargetInfluences[ i ];
	}
#endif`,e0=`#ifdef USE_MORPHTARGETS
	#ifndef USE_INSTANCING_MORPH
		uniform float morphTargetBaseInfluence;
		uniform float morphTargetInfluences[ MORPHTARGETS_COUNT ];
	#endif
	uniform sampler2DArray morphTargetsTexture;
	uniform ivec2 morphTargetsTextureSize;
	vec4 getMorph( const in int vertexIndex, const in int morphTargetIndex, const in int offset ) {
		int texelIndex = vertexIndex * MORPHTARGETS_TEXTURE_STRIDE + offset;
		int y = texelIndex / morphTargetsTextureSize.x;
		int x = texelIndex - y * morphTargetsTextureSize.x;
		ivec3 morphUV = ivec3( x, y, morphTargetIndex );
		return texelFetch( morphTargetsTexture, morphUV, 0 );
	}
#endif`,t0=`#ifdef USE_MORPHTARGETS
	transformed *= morphTargetBaseInfluence;
	for ( int i = 0; i < MORPHTARGETS_COUNT; i ++ ) {
		if ( morphTargetInfluences[ i ] != 0.0 ) transformed += getMorph( gl_VertexID, i, 0 ).xyz * morphTargetInfluences[ i ];
	}
#endif`,n0=`float faceDirection = gl_FrontFacing ? 1.0 : - 1.0;
#ifdef FLAT_SHADED
	vec3 fdx = dFdx( vViewPosition );
	vec3 fdy = dFdy( vViewPosition );
	vec3 normal = normalize( cross( fdx, fdy ) );
#else
	vec3 normal = normalize( vNormal );
	#ifdef DOUBLE_SIDED
		normal *= faceDirection;
	#endif
#endif
#if defined( USE_NORMALMAP_TANGENTSPACE ) || defined( USE_CLEARCOAT_NORMALMAP ) || defined( USE_ANISOTROPY )
	#ifdef USE_TANGENT
		mat3 tbn = mat3( normalize( vTangent ), normalize( vBitangent ), normal );
	#else
		mat3 tbn = getTangentFrame( - vViewPosition, normal,
		#if defined( USE_NORMALMAP )
			vNormalMapUv
		#elif defined( USE_CLEARCOAT_NORMALMAP )
			vClearcoatNormalMapUv
		#else
			vUv
		#endif
		);
	#endif
	#ifdef DOUBLE_SIDED
		tbn[0] *= faceDirection;
		tbn[1] *= faceDirection;
	#endif
#endif
#ifdef USE_CLEARCOAT_NORMALMAP
	#ifdef USE_TANGENT
		mat3 tbn2 = mat3( normalize( vTangent ), normalize( vBitangent ), normal );
	#else
		mat3 tbn2 = getTangentFrame( - vViewPosition, normal, vClearcoatNormalMapUv );
	#endif
	#ifdef DOUBLE_SIDED
		tbn2[0] *= faceDirection;
		tbn2[1] *= faceDirection;
	#endif
#endif
vec3 nonPerturbedNormal = normal;`,i0=`#ifdef USE_NORMALMAP_OBJECTSPACE
	normal = texture2D( normalMap, vNormalMapUv ).xyz * 2.0 - 1.0;
	#ifdef FLIP_SIDED
		normal = - normal;
	#endif
	#ifdef DOUBLE_SIDED
		normal = normal * faceDirection;
	#endif
	normal = normalize( normalMatrix * normal );
#elif defined( USE_NORMALMAP_TANGENTSPACE )
	vec3 mapN = texture2D( normalMap, vNormalMapUv ).xyz * 2.0 - 1.0;
	#if defined( USE_PACKED_NORMALMAP )
		mapN = vec3( mapN.xy, sqrt( saturate( 1.0 - dot( mapN.xy, mapN.xy ) ) ) );
	#endif
	mapN.xy *= normalScale;
	normal = normalize( tbn * mapN );
#elif defined( USE_BUMPMAP )
	normal = perturbNormalArb( - vViewPosition, normal, dHdxy_fwd(), faceDirection );
#endif`,s0=`#ifndef FLAT_SHADED
	varying vec3 vNormal;
	#ifdef USE_TANGENT
		varying vec3 vTangent;
		varying vec3 vBitangent;
	#endif
#endif`,r0=`#ifndef FLAT_SHADED
	varying vec3 vNormal;
	#ifdef USE_TANGENT
		varying vec3 vTangent;
		varying vec3 vBitangent;
	#endif
#endif`,o0=`#ifndef FLAT_SHADED
	vNormal = normalize( transformedNormal );
	#ifdef USE_TANGENT
		vTangent = normalize( transformedTangent );
		vBitangent = normalize( cross( vNormal, vTangent ) * tangent.w );
		#ifdef FLIP_SIDED
			vBitangent = - vBitangent;
		#endif
	#endif
#endif`,a0=`#ifdef USE_NORMALMAP
	uniform sampler2D normalMap;
	uniform vec2 normalScale;
#endif
#ifdef USE_NORMALMAP_OBJECTSPACE
	uniform mat3 normalMatrix;
#endif
#if ! defined ( USE_TANGENT ) && ( defined ( USE_NORMALMAP_TANGENTSPACE ) || defined ( USE_CLEARCOAT_NORMALMAP ) || defined( USE_ANISOTROPY ) )
	mat3 getTangentFrame( vec3 eye_pos, vec3 surf_norm, vec2 uv ) {
		vec3 q0 = dFdx( eye_pos.xyz );
		vec3 q1 = dFdy( eye_pos.xyz );
		vec2 st0 = dFdx( uv.st );
		vec2 st1 = dFdy( uv.st );
		vec3 N = surf_norm;
		vec3 q1perp = cross( q1, N );
		vec3 q0perp = cross( N, q0 );
		vec3 T = q1perp * st0.x + q0perp * st1.x;
		vec3 B = q1perp * st0.y + q0perp * st1.y;
		float det = max( dot( T, T ), dot( B, B ) );
		float scale = ( det == 0.0 ) ? 0.0 : inversesqrt( det );
		return mat3( T * scale, B * scale, N );
	}
#endif`,l0=`#ifdef USE_CLEARCOAT
	vec3 clearcoatNormal = nonPerturbedNormal;
#endif`,c0=`#ifdef USE_CLEARCOAT_NORMALMAP
	vec3 clearcoatMapN = texture2D( clearcoatNormalMap, vClearcoatNormalMapUv ).xyz * 2.0 - 1.0;
	clearcoatMapN.xy *= clearcoatNormalScale;
	clearcoatNormal = normalize( tbn2 * clearcoatMapN );
#endif`,h0=`#ifdef USE_CLEARCOATMAP
	uniform sampler2D clearcoatMap;
#endif
#ifdef USE_CLEARCOAT_NORMALMAP
	uniform sampler2D clearcoatNormalMap;
	uniform vec2 clearcoatNormalScale;
#endif
#ifdef USE_CLEARCOAT_ROUGHNESSMAP
	uniform sampler2D clearcoatRoughnessMap;
#endif`,u0=`#ifdef USE_IRIDESCENCEMAP
	uniform sampler2D iridescenceMap;
#endif
#ifdef USE_IRIDESCENCE_THICKNESSMAP
	uniform sampler2D iridescenceThicknessMap;
#endif`,f0=`#ifdef OPAQUE
diffuseColor.a = 1.0;
#endif
#ifdef USE_TRANSMISSION
diffuseColor.a *= material.transmissionAlpha;
#endif
gl_FragColor = vec4( outgoingLight, diffuseColor.a );`,d0=`vec3 packNormalToRGB( const in vec3 normal ) {
	return normalize( normal ) * 0.5 + 0.5;
}
vec3 unpackRGBToNormal( const in vec3 rgb ) {
	return 2.0 * rgb.xyz - 1.0;
}
const float PackUpscale = 256. / 255.;const float UnpackDownscale = 255. / 256.;const float ShiftRight8 = 1. / 256.;
const float Inv255 = 1. / 255.;
const vec4 PackFactors = vec4( 1.0, 256.0, 256.0 * 256.0, 256.0 * 256.0 * 256.0 );
const vec2 UnpackFactors2 = vec2( UnpackDownscale, 1.0 / PackFactors.g );
const vec3 UnpackFactors3 = vec3( UnpackDownscale / PackFactors.rg, 1.0 / PackFactors.b );
const vec4 UnpackFactors4 = vec4( UnpackDownscale / PackFactors.rgb, 1.0 / PackFactors.a );
vec4 packDepthToRGBA( const in float v ) {
	if( v <= 0.0 )
		return vec4( 0., 0., 0., 0. );
	if( v >= 1.0 )
		return vec4( 1., 1., 1., 1. );
	float vuf;
	float af = modf( v * PackFactors.a, vuf );
	float bf = modf( vuf * ShiftRight8, vuf );
	float gf = modf( vuf * ShiftRight8, vuf );
	return vec4( vuf * Inv255, gf * PackUpscale, bf * PackUpscale, af );
}
vec3 packDepthToRGB( const in float v ) {
	if( v <= 0.0 )
		return vec3( 0., 0., 0. );
	if( v >= 1.0 )
		return vec3( 1., 1., 1. );
	float vuf;
	float bf = modf( v * PackFactors.b, vuf );
	float gf = modf( vuf * ShiftRight8, vuf );
	return vec3( vuf * Inv255, gf * PackUpscale, bf );
}
vec2 packDepthToRG( const in float v ) {
	if( v <= 0.0 )
		return vec2( 0., 0. );
	if( v >= 1.0 )
		return vec2( 1., 1. );
	float vuf;
	float gf = modf( v * 256., vuf );
	return vec2( vuf * Inv255, gf );
}
float unpackRGBAToDepth( const in vec4 v ) {
	return dot( v, UnpackFactors4 );
}
float unpackRGBToDepth( const in vec3 v ) {
	return dot( v, UnpackFactors3 );
}
float unpackRGToDepth( const in vec2 v ) {
	return v.r * UnpackFactors2.r + v.g * UnpackFactors2.g;
}
vec4 pack2HalfToRGBA( const in vec2 v ) {
	vec4 r = vec4( v.x, fract( v.x * 255.0 ), v.y, fract( v.y * 255.0 ) );
	return vec4( r.x - r.y / 255.0, r.y, r.z - r.w / 255.0, r.w );
}
vec2 unpackRGBATo2Half( const in vec4 v ) {
	return vec2( v.x + ( v.y / 255.0 ), v.z + ( v.w / 255.0 ) );
}
float viewZToOrthographicDepth( const in float viewZ, const in float near, const in float far ) {
	return ( viewZ + near ) / ( near - far );
}
float orthographicDepthToViewZ( const in float depth, const in float near, const in float far ) {
	#ifdef USE_REVERSED_DEPTH_BUFFER
	
		return depth * ( far - near ) - far;
	#else
		return depth * ( near - far ) - near;
	#endif
}
float viewZToPerspectiveDepth( const in float viewZ, const in float near, const in float far ) {
	return ( ( near + viewZ ) * far ) / ( ( far - near ) * viewZ );
}
float perspectiveDepthToViewZ( const in float depth, const in float near, const in float far ) {
	
	#ifdef USE_REVERSED_DEPTH_BUFFER
		return ( near * far ) / ( ( near - far ) * depth - near );
	#else
		return ( near * far ) / ( ( far - near ) * depth - far );
	#endif
}`,p0=`#ifdef PREMULTIPLIED_ALPHA
	gl_FragColor.rgb *= gl_FragColor.a;
#endif`,m0=`vec4 mvPosition = vec4( transformed, 1.0 );
#ifdef USE_BATCHING
	mvPosition = batchingMatrix * mvPosition;
#endif
#ifdef USE_INSTANCING
	mvPosition = instanceMatrix * mvPosition;
#endif
mvPosition = modelViewMatrix * mvPosition;
gl_Position = projectionMatrix * mvPosition;`,g0=`#ifdef DITHERING
	gl_FragColor.rgb = dithering( gl_FragColor.rgb );
#endif`,_0=`#ifdef DITHERING
	vec3 dithering( vec3 color ) {
		float grid_position = rand( gl_FragCoord.xy );
		vec3 dither_shift_RGB = vec3( 0.25 / 255.0, -0.25 / 255.0, 0.25 / 255.0 );
		dither_shift_RGB = mix( 2.0 * dither_shift_RGB, -2.0 * dither_shift_RGB, grid_position );
		return color + dither_shift_RGB;
	}
#endif`,x0=`float roughnessFactor = roughness;
#ifdef USE_ROUGHNESSMAP
	vec4 texelRoughness = texture2D( roughnessMap, vRoughnessMapUv );
	roughnessFactor *= texelRoughness.g;
#endif`,y0=`#ifdef USE_ROUGHNESSMAP
	uniform sampler2D roughnessMap;
#endif`,v0=`#if NUM_SPOT_LIGHT_COORDS > 0
	varying vec4 vSpotLightCoord[ NUM_SPOT_LIGHT_COORDS ];
#endif
#if NUM_SPOT_LIGHT_MAPS > 0
	uniform sampler2D spotLightMap[ NUM_SPOT_LIGHT_MAPS ];
#endif
#ifdef USE_SHADOWMAP
	#if NUM_SUN_LIGHT_SHADOWS > 0
		#define SUN_LIGHT_CASCADES 2
		#if defined( SHADOWMAP_TYPE_PCF )
			uniform sampler2DShadow sunShadowMap[ NUM_SUN_LIGHT_SHADOWS ];
		#else
			uniform sampler2D sunShadowMap[ NUM_SUN_LIGHT_SHADOWS ];
		#endif
		uniform mat4 sunShadowMatrix[ NUM_SUN_LIGHT_SHADOWS * SUN_LIGHT_CASCADES ];
		uniform vec4 sunShadowCascade[ NUM_SUN_LIGHT_SHADOWS * SUN_LIGHT_CASCADES ];
		varying vec4 vSunShadowWorldPosition;
		varying vec3 vSunShadowWorldNormal;
		struct SunLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};
		uniform SunLightShadow sunLightShadows[ NUM_SUN_LIGHT_SHADOWS ];
	#endif
	#if NUM_DIR_LIGHT_SHADOWS > 0
		#if defined( SHADOWMAP_TYPE_PCF )
			uniform sampler2DShadow directionalShadowMap[ NUM_DIR_LIGHT_SHADOWS ];
		#else
			uniform sampler2D directionalShadowMap[ NUM_DIR_LIGHT_SHADOWS ];
		#endif
		varying vec4 vDirectionalShadowCoord[ NUM_DIR_LIGHT_SHADOWS ];
		struct DirectionalLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};
		uniform DirectionalLightShadow directionalLightShadows[ NUM_DIR_LIGHT_SHADOWS ];
	#endif
	#if NUM_SPOT_LIGHT_SHADOWS > 0
		#if defined( SHADOWMAP_TYPE_PCF )
			uniform sampler2DShadow spotShadowMap[ NUM_SPOT_LIGHT_SHADOWS ];
		#else
			uniform sampler2D spotShadowMap[ NUM_SPOT_LIGHT_SHADOWS ];
		#endif
		struct SpotLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};
		uniform SpotLightShadow spotLightShadows[ NUM_SPOT_LIGHT_SHADOWS ];
	#endif
	#if NUM_POINT_LIGHT_SHADOWS > 0
		#if defined( SHADOWMAP_TYPE_PCF )
			uniform samplerCubeShadow pointShadowMap[ NUM_POINT_LIGHT_SHADOWS ];
		#elif defined( SHADOWMAP_TYPE_BASIC )
			uniform samplerCube pointShadowMap[ NUM_POINT_LIGHT_SHADOWS ];
		#endif
		varying vec4 vPointShadowCoord[ NUM_POINT_LIGHT_SHADOWS ];
		struct PointLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
			float shadowCameraNear;
			float shadowCameraFar;
		};
		uniform PointLightShadow pointLightShadows[ NUM_POINT_LIGHT_SHADOWS ];
	#endif
	#if defined( SHADOWMAP_TYPE_PCF )
		float interleavedGradientNoise( vec2 position ) {
			return fract( 52.9829189 * fract( dot( position, vec2( 0.06711056, 0.00583715 ) ) ) );
		}
		vec2 vogelDiskSample( int sampleIndex, int samplesCount, float phi ) {
			const float goldenAngle = 2.399963229728653;
			float r = sqrt( ( float( sampleIndex ) + 0.5 ) / float( samplesCount ) );
			float theta = float( sampleIndex ) * goldenAngle + phi;
			return vec2( cos( theta ), sin( theta ) ) * r;
		}
	#endif
	#if defined( SHADOWMAP_TYPE_PCF )
		float getShadow( sampler2DShadow shadowMap, vec2 shadowMapSize, float shadowIntensity, float shadowBias, float shadowRadius, vec4 shadowCoord ) {
			float shadow = 1.0;
			shadowCoord.xyz /= shadowCoord.w;
			#ifdef USE_REVERSED_DEPTH_BUFFER
				shadowCoord.z -= shadowBias;
			#else
				shadowCoord.z += shadowBias;
			#endif
			bool inFrustum = shadowCoord.x >= 0.0 && shadowCoord.x <= 1.0 && shadowCoord.y >= 0.0 && shadowCoord.y <= 1.0;
			bool frustumTest = inFrustum && shadowCoord.z <= 1.0;
			if ( frustumTest ) {
				vec2 texelSize = vec2( 1.0 ) / shadowMapSize;
				vec2 radius = shadowRadius * texelSize;
				float phi = interleavedGradientNoise( gl_FragCoord.xy ) * PI2;
				shadow = (
					texture( shadowMap, vec3( shadowCoord.xy + vogelDiskSample( 0, 5, phi ) * radius, shadowCoord.z ) ) +
					texture( shadowMap, vec3( shadowCoord.xy + vogelDiskSample( 1, 5, phi ) * radius, shadowCoord.z ) ) +
					texture( shadowMap, vec3( shadowCoord.xy + vogelDiskSample( 2, 5, phi ) * radius, shadowCoord.z ) ) +
					texture( shadowMap, vec3( shadowCoord.xy + vogelDiskSample( 3, 5, phi ) * radius, shadowCoord.z ) ) +
					texture( shadowMap, vec3( shadowCoord.xy + vogelDiskSample( 4, 5, phi ) * radius, shadowCoord.z ) )
				) * 0.2;
			}
			return mix( 1.0, shadow, shadowIntensity );
		}
	#elif defined( SHADOWMAP_TYPE_VSM )
		float getShadow( sampler2D shadowMap, vec2 shadowMapSize, float shadowIntensity, float shadowBias, float shadowRadius, vec4 shadowCoord ) {
			float shadow = 1.0;
			shadowCoord.xyz /= shadowCoord.w;
			#ifdef USE_REVERSED_DEPTH_BUFFER
				shadowCoord.z -= shadowBias;
			#else
				shadowCoord.z += shadowBias;
			#endif
			bool inFrustum = shadowCoord.x >= 0.0 && shadowCoord.x <= 1.0 && shadowCoord.y >= 0.0 && shadowCoord.y <= 1.0;
			bool frustumTest = inFrustum && shadowCoord.z <= 1.0;
			if ( frustumTest ) {
				vec2 distribution = texture2D( shadowMap, shadowCoord.xy ).rg;
				float mean = distribution.x;
				float variance = distribution.y * distribution.y;
				#ifdef USE_REVERSED_DEPTH_BUFFER
					float hard_shadow = step( mean, shadowCoord.z );
				#else
					float hard_shadow = step( shadowCoord.z, mean );
				#endif
				
				if ( hard_shadow == 1.0 ) {
					shadow = 1.0;
				} else {
					variance = max( variance, 0.0000001 );
					float d = shadowCoord.z - mean;
					float p_max = variance / ( variance + d * d );
					p_max = clamp( ( p_max - 0.3 ) / 0.65, 0.0, 1.0 );
					shadow = max( hard_shadow, p_max );
				}
			}
			return mix( 1.0, shadow, shadowIntensity );
		}
	#else
		float getShadow( sampler2D shadowMap, vec2 shadowMapSize, float shadowIntensity, float shadowBias, float shadowRadius, vec4 shadowCoord ) {
			float shadow = 1.0;
			shadowCoord.xyz /= shadowCoord.w;
			#ifdef USE_REVERSED_DEPTH_BUFFER
				shadowCoord.z -= shadowBias;
			#else
				shadowCoord.z += shadowBias;
			#endif
			bool inFrustum = shadowCoord.x >= 0.0 && shadowCoord.x <= 1.0 && shadowCoord.y >= 0.0 && shadowCoord.y <= 1.0;
			bool frustumTest = inFrustum && shadowCoord.z <= 1.0;
			if ( frustumTest ) {
				float depth = texture2D( shadowMap, shadowCoord.xy ).r;
				#ifdef USE_REVERSED_DEPTH_BUFFER
					shadow = step( depth, shadowCoord.z );
				#else
					shadow = step( shadowCoord.z, depth );
				#endif
			}
			return mix( 1.0, shadow, shadowIntensity );
		}
	#endif
	#if NUM_SUN_LIGHT_SHADOWS > 0
		float getSunShadow(
			#if defined( SHADOWMAP_TYPE_PCF )
				sampler2DShadow shadowMap,
			#else
				sampler2D shadowMap,
			#endif
			SunLightShadow sunLightShadow,
			int shadowIndex
		) {
			vec4 shadowWorldPosition = vec4( vSunShadowWorldPosition.xyz + vSunShadowWorldNormal * sunLightShadow.shadowNormalBias, 1.0 );
			float viewDepth = vSunShadowWorldPosition.w;
			int cascadeOffset = shadowIndex * SUN_LIGHT_CASCADES;
			float shadow = 1.0;
			for ( int i = SUN_LIGHT_CASCADES - 1; i >= 0; i -- ) {
				vec4 cascade = sunShadowCascade[ cascadeOffset + i ];
				if ( viewDepth >= cascade.x && viewDepth < cascade.y ) {
					float cascadeShadow = getShadow(
						shadowMap,
						sunLightShadow.shadowMapSize,
						sunLightShadow.shadowIntensity,
						sunLightShadow.shadowBias,
						sunLightShadow.shadowRadius,
						sunShadowMatrix[ cascadeOffset + i ] * shadowWorldPosition
					);
					shadow = mix( cascadeShadow, shadow, smoothstep( cascade.z, cascade.y, viewDepth ) );
				}
			}
			return shadow;
		}
	#endif
	#if NUM_POINT_LIGHT_SHADOWS > 0
	#if defined( SHADOWMAP_TYPE_PCF )
	float getPointShadow( samplerCubeShadow shadowMap, vec2 shadowMapSize, float shadowIntensity, float shadowBias, float shadowRadius, vec4 shadowCoord, float shadowCameraNear, float shadowCameraFar ) {
		float shadow = 1.0;
		vec3 lightToPosition = shadowCoord.xyz;
		vec3 bd3D = normalize( lightToPosition );
		vec3 absVec = abs( lightToPosition );
		float viewSpaceZ = max( max( absVec.x, absVec.y ), absVec.z );
		if ( viewSpaceZ - shadowCameraFar <= 0.0 && viewSpaceZ - shadowCameraNear >= 0.0 ) {
			float lightDistance = length( lightToPosition );
			float depthRange = shadowCameraFar - shadowCameraNear;
			#ifdef USE_REVERSED_DEPTH_BUFFER
				float depthA = - shadowCameraNear / depthRange - shadowBias;
				float depthB = shadowCameraNear * shadowCameraFar / ( depthRange * lightDistance );
			#else
				float depthA = shadowCameraFar / depthRange + shadowBias;
				float depthB = - shadowCameraNear * shadowCameraFar / ( depthRange * lightDistance );
			#endif
			float texelSize = shadowRadius / shadowMapSize.x;
			vec3 absDir = abs( bd3D );
			vec3 tangent = absDir.x > absDir.z ? vec3( 0.0, 1.0, 0.0 ) : vec3( 1.0, 0.0, 0.0 );
			tangent = normalize( cross( bd3D, tangent ) );
			vec3 bitangent = cross( bd3D, tangent );
			float phi = interleavedGradientNoise( gl_FragCoord.xy ) * PI2;
			shadow = 0.0;
			for ( int i = 0; i < 5; i ++ ) {
				vec2 offset = vogelDiskSample( i, 5, phi );
				vec3 direction = bd3D + ( tangent * offset.x + bitangent * offset.y ) * texelSize;
				vec3 absDirection = abs( direction );
				float dp = depthA + depthB / max( max( absDirection.x, absDirection.y ), absDirection.z );
				shadow += texture( shadowMap, vec4( direction, dp ) );
			}
			shadow *= 0.2;
		}
		return mix( 1.0, shadow, shadowIntensity );
	}
	#elif defined( SHADOWMAP_TYPE_BASIC )
	float getPointShadow( samplerCube shadowMap, vec2 shadowMapSize, float shadowIntensity, float shadowBias, float shadowRadius, vec4 shadowCoord, float shadowCameraNear, float shadowCameraFar ) {
		float shadow = 1.0;
		vec3 lightToPosition = shadowCoord.xyz;
		vec3 absVec = abs( lightToPosition );
		float viewSpaceZ = max( max( absVec.x, absVec.y ), absVec.z );
		if ( viewSpaceZ - shadowCameraFar <= 0.0 && viewSpaceZ - shadowCameraNear >= 0.0 ) {
			float dp = ( shadowCameraFar * ( viewSpaceZ - shadowCameraNear ) ) / ( viewSpaceZ * ( shadowCameraFar - shadowCameraNear ) );
			dp += shadowBias;
			vec3 bd3D = normalize( lightToPosition );
			float depth = textureCube( shadowMap, bd3D ).r;
			#ifdef USE_REVERSED_DEPTH_BUFFER
				depth = 1.0 - depth;
			#endif
			shadow = step( dp, depth );
		}
		return mix( 1.0, shadow, shadowIntensity );
	}
	#endif
	#endif
#endif`,S0=`#if NUM_SPOT_LIGHT_COORDS > 0
	uniform mat4 spotLightMatrix[ NUM_SPOT_LIGHT_COORDS ];
	varying vec4 vSpotLightCoord[ NUM_SPOT_LIGHT_COORDS ];
#endif
#ifdef USE_SHADOWMAP
	#if NUM_SUN_LIGHT_SHADOWS > 0
		varying vec4 vSunShadowWorldPosition;
		varying vec3 vSunShadowWorldNormal;
	#endif
	#if NUM_DIR_LIGHT_SHADOWS > 0
		uniform mat4 directionalShadowMatrix[ NUM_DIR_LIGHT_SHADOWS ];
		varying vec4 vDirectionalShadowCoord[ NUM_DIR_LIGHT_SHADOWS ];
		struct DirectionalLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};
		uniform DirectionalLightShadow directionalLightShadows[ NUM_DIR_LIGHT_SHADOWS ];
	#endif
	#if NUM_SPOT_LIGHT_SHADOWS > 0
		struct SpotLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
		};
		uniform SpotLightShadow spotLightShadows[ NUM_SPOT_LIGHT_SHADOWS ];
	#endif
	#if NUM_POINT_LIGHT_SHADOWS > 0
		uniform mat4 pointShadowMatrix[ NUM_POINT_LIGHT_SHADOWS ];
		varying vec4 vPointShadowCoord[ NUM_POINT_LIGHT_SHADOWS ];
		struct PointLightShadow {
			float shadowIntensity;
			float shadowBias;
			float shadowNormalBias;
			float shadowRadius;
			vec2 shadowMapSize;
			float shadowCameraNear;
			float shadowCameraFar;
		};
		uniform PointLightShadow pointLightShadows[ NUM_POINT_LIGHT_SHADOWS ];
	#endif
#endif`,M0=`#if ( defined( USE_SHADOWMAP ) && ( NUM_DIR_LIGHT_SHADOWS > 0 || NUM_SUN_LIGHT_SHADOWS > 0 || NUM_POINT_LIGHT_SHADOWS > 0 ) ) || ( NUM_SPOT_LIGHT_COORDS > 0 )
	#ifdef HAS_NORMAL
		vec3 shadowWorldNormal = transformNormalByInverseViewMatrix( transformedNormal, viewMatrix );
	#else
		vec3 shadowWorldNormal = vec3( 0.0 );
	#endif
	vec4 shadowWorldPosition;
#endif
#if defined( USE_SHADOWMAP )
	#if NUM_SUN_LIGHT_SHADOWS > 0
		vSunShadowWorldPosition = vec4( worldPosition.xyz, - mvPosition.z );
		vSunShadowWorldNormal = shadowWorldNormal;
	#endif
	#if NUM_DIR_LIGHT_SHADOWS > 0
		#pragma unroll_loop_start
		for ( int i = 0; i < NUM_DIR_LIGHT_SHADOWS; i ++ ) {
			shadowWorldPosition = worldPosition + vec4( shadowWorldNormal * directionalLightShadows[ i ].shadowNormalBias, 0 );
			vDirectionalShadowCoord[ i ] = directionalShadowMatrix[ i ] * shadowWorldPosition;
		}
		#pragma unroll_loop_end
	#endif
	#if NUM_POINT_LIGHT_SHADOWS > 0
		#pragma unroll_loop_start
		for ( int i = 0; i < NUM_POINT_LIGHT_SHADOWS; i ++ ) {
			shadowWorldPosition = worldPosition + vec4( shadowWorldNormal * pointLightShadows[ i ].shadowNormalBias, 0 );
			vPointShadowCoord[ i ] = pointShadowMatrix[ i ] * shadowWorldPosition;
		}
		#pragma unroll_loop_end
	#endif
#endif
#if NUM_SPOT_LIGHT_COORDS > 0
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_SPOT_LIGHT_COORDS; i ++ ) {
		shadowWorldPosition = worldPosition;
		#if ( defined( USE_SHADOWMAP ) && UNROLLED_LOOP_INDEX < NUM_SPOT_LIGHT_SHADOWS )
			shadowWorldPosition.xyz += shadowWorldNormal * spotLightShadows[ i ].shadowNormalBias;
		#endif
		vSpotLightCoord[ i ] = spotLightMatrix[ i ] * shadowWorldPosition;
	}
	#pragma unroll_loop_end
#endif`,b0=`float getShadowMask() {
	float shadow = 1.0;
	#ifdef USE_SHADOWMAP
	#if NUM_SUN_LIGHT_SHADOWS > 0
	SunLightShadow sunLight;
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_SUN_LIGHT_SHADOWS; i ++ ) {
		sunLight = sunLightShadows[ i ];
		shadow *= receiveShadow ? getSunShadow( sunShadowMap[ i ], sunLight, UNROLLED_LOOP_INDEX ) : 1.0;
	}
	#pragma unroll_loop_end
	#endif
	#if NUM_DIR_LIGHT_SHADOWS > 0
	DirectionalLightShadow directionalLight;
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_DIR_LIGHT_SHADOWS; i ++ ) {
		directionalLight = directionalLightShadows[ i ];
		shadow *= receiveShadow ? getShadow( directionalShadowMap[ i ], directionalLight.shadowMapSize, directionalLight.shadowIntensity, directionalLight.shadowBias, directionalLight.shadowRadius, vDirectionalShadowCoord[ i ] ) : 1.0;
	}
	#pragma unroll_loop_end
	#endif
	#if NUM_SPOT_LIGHT_SHADOWS > 0
	SpotLightShadow spotLight;
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_SPOT_LIGHT_SHADOWS; i ++ ) {
		spotLight = spotLightShadows[ i ];
		shadow *= receiveShadow ? getShadow( spotShadowMap[ i ], spotLight.shadowMapSize, spotLight.shadowIntensity, spotLight.shadowBias, spotLight.shadowRadius, vSpotLightCoord[ i ] ) : 1.0;
	}
	#pragma unroll_loop_end
	#endif
	#if NUM_POINT_LIGHT_SHADOWS > 0 && ( defined( SHADOWMAP_TYPE_PCF ) || defined( SHADOWMAP_TYPE_BASIC ) )
	PointLightShadow pointLight;
	#pragma unroll_loop_start
	for ( int i = 0; i < NUM_POINT_LIGHT_SHADOWS; i ++ ) {
		pointLight = pointLightShadows[ i ];
		shadow *= receiveShadow ? getPointShadow( pointShadowMap[ i ], pointLight.shadowMapSize, pointLight.shadowIntensity, pointLight.shadowBias, pointLight.shadowRadius, vPointShadowCoord[ i ], pointLight.shadowCameraNear, pointLight.shadowCameraFar ) : 1.0;
	}
	#pragma unroll_loop_end
	#endif
	#endif
	return shadow;
}`,w0=`#ifdef USE_SKINNING
	mat4 boneMatX = getBoneMatrix( skinIndex.x );
	mat4 boneMatY = getBoneMatrix( skinIndex.y );
	mat4 boneMatZ = getBoneMatrix( skinIndex.z );
	mat4 boneMatW = getBoneMatrix( skinIndex.w );
#endif`,A0=`#ifdef USE_SKINNING
	uniform mat4 bindMatrix;
	uniform mat4 bindMatrixInverse;
	uniform highp sampler2D boneTexture;
	mat4 getBoneMatrix( const in float i ) {
		int size = textureSize( boneTexture, 0 ).x;
		int j = int( i ) * 4;
		int x = j % size;
		int y = j / size;
		vec4 v1 = texelFetch( boneTexture, ivec2( x, y ), 0 );
		vec4 v2 = texelFetch( boneTexture, ivec2( x + 1, y ), 0 );
		vec4 v3 = texelFetch( boneTexture, ivec2( x + 2, y ), 0 );
		vec4 v4 = texelFetch( boneTexture, ivec2( x + 3, y ), 0 );
		return mat4( v1, v2, v3, v4 );
	}
#endif`,T0=`#ifdef USE_SKINNING
	vec4 skinVertex = bindMatrix * vec4( transformed, 1.0 );
	vec4 skinned = vec4( 0.0 );
	skinned += boneMatX * skinVertex * skinWeight.x;
	skinned += boneMatY * skinVertex * skinWeight.y;
	skinned += boneMatZ * skinVertex * skinWeight.z;
	skinned += boneMatW * skinVertex * skinWeight.w;
	transformed = ( bindMatrixInverse * skinned ).xyz;
#endif`,E0=`#ifdef USE_SKINNING
	mat4 skinMatrix = mat4( 0.0 );
	skinMatrix += skinWeight.x * boneMatX;
	skinMatrix += skinWeight.y * boneMatY;
	skinMatrix += skinWeight.z * boneMatZ;
	skinMatrix += skinWeight.w * boneMatW;
	skinMatrix = bindMatrixInverse * skinMatrix * bindMatrix;
	objectNormal = vec4( skinMatrix * vec4( objectNormal, 0.0 ) ).xyz;
	#ifdef USE_TANGENT
		objectTangent = vec4( skinMatrix * vec4( objectTangent, 0.0 ) ).xyz;
	#endif
#endif`,R0=`float specularStrength;
#ifdef USE_SPECULARMAP
	vec4 texelSpecular = texture2D( specularMap, vSpecularMapUv );
	specularStrength = texelSpecular.r;
#else
	specularStrength = 1.0;
#endif`,C0=`#ifdef USE_SPECULARMAP
	uniform sampler2D specularMap;
#endif`,P0=`#if defined( TONE_MAPPING )
	gl_FragColor.rgb = toneMapping( gl_FragColor.rgb );
#endif`,I0=`#ifndef saturate
#define saturate( a ) clamp( a, 0.0, 1.0 )
#endif
uniform float toneMappingExposure;
vec3 LinearToneMapping( vec3 color ) {
	return saturate( toneMappingExposure * color );
}
vec3 ReinhardToneMapping( vec3 color ) {
	color *= toneMappingExposure;
	return saturate( color / ( vec3( 1.0 ) + color ) );
}
vec3 CineonToneMapping( vec3 color ) {
	color *= toneMappingExposure;
	color = max( vec3( 0.0 ), color - 0.004 );
	return pow( ( color * ( 6.2 * color + 0.5 ) ) / ( color * ( 6.2 * color + 1.7 ) + 0.06 ), vec3( 2.2 ) );
}
vec3 RRTAndODTFit( vec3 v ) {
	vec3 a = v * ( v + 0.0245786 ) - 0.000090537;
	vec3 b = v * ( 0.983729 * v + 0.4329510 ) + 0.238081;
	return a / b;
}
vec3 ACESFilmicToneMapping( vec3 color ) {
	const mat3 ACESInputMat = mat3(
		vec3( 0.59719, 0.07600, 0.02840 ),		vec3( 0.35458, 0.90834, 0.13383 ),
		vec3( 0.04823, 0.01566, 0.83777 )
	);
	const mat3 ACESOutputMat = mat3(
		vec3(  1.60475, -0.10208, -0.00327 ),		vec3( -0.53108,  1.10813, -0.07276 ),
		vec3( -0.07367, -0.00605,  1.07602 )
	);
	color *= toneMappingExposure / 0.6;
	color = ACESInputMat * color;
	color = RRTAndODTFit( color );
	color = ACESOutputMat * color;
	return saturate( color );
}
const mat3 LINEAR_REC2020_TO_LINEAR_SRGB = mat3(
	vec3( 1.6605, - 0.1246, - 0.0182 ),
	vec3( - 0.5876, 1.1329, - 0.1006 ),
	vec3( - 0.0728, - 0.0083, 1.1187 )
);
const mat3 LINEAR_SRGB_TO_LINEAR_REC2020 = mat3(
	vec3( 0.6274, 0.0691, 0.0164 ),
	vec3( 0.3293, 0.9195, 0.0880 ),
	vec3( 0.0433, 0.0113, 0.8956 )
);
vec3 agxDefaultContrastApprox( vec3 x ) {
	vec3 x2 = x * x;
	vec3 x4 = x2 * x2;
	return + 15.5 * x4 * x2
		- 40.14 * x4 * x
		+ 31.96 * x4
		- 6.868 * x2 * x
		+ 0.4298 * x2
		+ 0.1191 * x
		- 0.00232;
}
vec3 AgXToneMapping( vec3 color ) {
	const mat3 AgXInsetMatrix = mat3(
		vec3( 0.856627153315983, 0.137318972929847, 0.11189821299995 ),
		vec3( 0.0951212405381588, 0.761241990602591, 0.0767994186031903 ),
		vec3( 0.0482516061458583, 0.101439036467562, 0.811302368396859 )
	);
	const mat3 AgXOutsetMatrix = mat3(
		vec3( 1.1271005818144368, - 0.1413297634984383, - 0.14132976349843826 ),
		vec3( - 0.11060664309660323, 1.157823702216272, - 0.11060664309660294 ),
		vec3( - 0.016493938717834573, - 0.016493938717834257, 1.2519364065950405 )
	);
	const float AgxMinEv = - 12.47393;	const float AgxMaxEv = 4.026069;
	color *= toneMappingExposure;
	color = LINEAR_SRGB_TO_LINEAR_REC2020 * color;
	color = AgXInsetMatrix * color;
	color = max( color, 1e-10 );	color = log2( color );
	color = ( color - AgxMinEv ) / ( AgxMaxEv - AgxMinEv );
	color = clamp( color, 0.0, 1.0 );
	color = agxDefaultContrastApprox( color );
	color = AgXOutsetMatrix * color;
	color = pow( max( vec3( 0.0 ), color ), vec3( 2.2 ) );
	color = LINEAR_REC2020_TO_LINEAR_SRGB * color;
	color = clamp( color, 0.0, 1.0 );
	return color;
}
vec3 NeutralToneMapping( vec3 color ) {
	const float StartCompression = 0.8 - 0.04;
	const float Desaturation = 0.15;
	color *= toneMappingExposure;
	float x = min( color.r, min( color.g, color.b ) );
	float offset = x < 0.08 ? x - 6.25 * x * x : 0.04;
	color -= offset;
	float peak = max( color.r, max( color.g, color.b ) );
	if ( peak < StartCompression ) return color;
	float d = 1. - StartCompression;
	float newPeak = 1. - d * d / ( peak + d - StartCompression );
	color *= newPeak / peak;
	float g = 1. - 1. / ( Desaturation * ( peak - newPeak ) + 1. );
	return mix( color, vec3( newPeak ), g );
}
vec3 CustomToneMapping( vec3 color ) { return color; }`,D0=`#ifdef USE_TRANSMISSION
	material.transmission = transmission;
	material.transmissionAlpha = 1.0;
	material.thickness = thickness;
	material.attenuationDistance = attenuationDistance;
	material.attenuationColor = attenuationColor;
	#ifdef USE_TRANSMISSIONMAP
		material.transmission *= texture2D( transmissionMap, vTransmissionMapUv ).r;
	#endif
	#ifdef USE_THICKNESSMAP
		material.thickness *= texture2D( thicknessMap, vThicknessMapUv ).g;
	#endif
	vec3 pos = vWorldPosition;
	vec3 v = normalize( cameraPosition - pos );
	vec3 n = transformNormalByInverseViewMatrix( normal, viewMatrix );
	vec4 transmitted = getIBLVolumeRefraction(
		n, v, material.roughness, material.diffuseContribution, material.specularColorBlended, material.specularF90,
		pos, modelMatrix, viewMatrix, projectionMatrix, material.dispersion, material.ior, material.thickness,
		material.attenuationColor, material.attenuationDistance );
	material.transmissionAlpha = mix( material.transmissionAlpha, transmitted.a, material.transmission );
	totalDiffuse = mix( totalDiffuse, transmitted.rgb, material.transmission );
#endif`,L0=`#ifdef USE_TRANSMISSION
	uniform float transmission;
	uniform float thickness;
	uniform float attenuationDistance;
	uniform vec3 attenuationColor;
	#ifdef USE_TRANSMISSIONMAP
		uniform sampler2D transmissionMap;
	#endif
	#ifdef USE_THICKNESSMAP
		uniform sampler2D thicknessMap;
	#endif
	uniform vec2 transmissionSamplerSize;
	uniform sampler2D transmissionSamplerMap;
	uniform mat4 modelMatrix;
	uniform mat4 projectionMatrix;
	varying vec3 vWorldPosition;
	float w0( float a ) {
		return ( 1.0 / 6.0 ) * ( a * ( a * ( - a + 3.0 ) - 3.0 ) + 1.0 );
	}
	float w1( float a ) {
		return ( 1.0 / 6.0 ) * ( a *  a * ( 3.0 * a - 6.0 ) + 4.0 );
	}
	float w2( float a ){
		return ( 1.0 / 6.0 ) * ( a * ( a * ( - 3.0 * a + 3.0 ) + 3.0 ) + 1.0 );
	}
	float w3( float a ) {
		return ( 1.0 / 6.0 ) * ( a * a * a );
	}
	float g0( float a ) {
		return w0( a ) + w1( a );
	}
	float g1( float a ) {
		return w2( a ) + w3( a );
	}
	float h0( float a ) {
		return - 1.0 + w1( a ) / ( w0( a ) + w1( a ) );
	}
	float h1( float a ) {
		return 1.0 + w3( a ) / ( w2( a ) + w3( a ) );
	}
	vec4 bicubic( sampler2D tex, vec2 uv, vec4 texelSize, float lod ) {
		uv = uv * texelSize.zw + 0.5;
		vec2 iuv = floor( uv );
		vec2 fuv = fract( uv );
		float g0x = g0( fuv.x );
		float g1x = g1( fuv.x );
		float h0x = h0( fuv.x );
		float h1x = h1( fuv.x );
		float h0y = h0( fuv.y );
		float h1y = h1( fuv.y );
		vec2 p0 = ( vec2( iuv.x + h0x, iuv.y + h0y ) - 0.5 ) * texelSize.xy;
		vec2 p1 = ( vec2( iuv.x + h1x, iuv.y + h0y ) - 0.5 ) * texelSize.xy;
		vec2 p2 = ( vec2( iuv.x + h0x, iuv.y + h1y ) - 0.5 ) * texelSize.xy;
		vec2 p3 = ( vec2( iuv.x + h1x, iuv.y + h1y ) - 0.5 ) * texelSize.xy;
		return g0( fuv.y ) * ( g0x * textureLod( tex, p0, lod ) + g1x * textureLod( tex, p1, lod ) ) +
			g1( fuv.y ) * ( g0x * textureLod( tex, p2, lod ) + g1x * textureLod( tex, p3, lod ) );
	}
	vec4 textureBicubic( sampler2D sampler, vec2 uv, float lod ) {
		vec2 fLodSize = vec2( textureSize( sampler, int( lod ) ) );
		vec2 cLodSize = vec2( textureSize( sampler, int( lod + 1.0 ) ) );
		vec2 fLodSizeInv = 1.0 / fLodSize;
		vec2 cLodSizeInv = 1.0 / cLodSize;
		vec4 fSample = bicubic( sampler, uv, vec4( fLodSizeInv, fLodSize ), floor( lod ) );
		vec4 cSample = bicubic( sampler, uv, vec4( cLodSizeInv, cLodSize ), ceil( lod ) );
		return mix( fSample, cSample, fract( lod ) );
	}
	vec3 getVolumeTransmissionRay( const in vec3 n, const in vec3 v, const in float thickness, const in float ior, const in mat4 modelMatrix ) {
		vec3 refractionVector = refract( - v, normalize( n ), 1.0 / ior );
		vec3 modelScale;
		modelScale.x = length( vec3( modelMatrix[ 0 ].xyz ) );
		modelScale.y = length( vec3( modelMatrix[ 1 ].xyz ) );
		modelScale.z = length( vec3( modelMatrix[ 2 ].xyz ) );
		return normalize( refractionVector ) * thickness * modelScale;
	}
	float applyIorToRoughness( const in float roughness, const in float ior ) {
		return roughness * clamp( ior * 2.0 - 2.0, 0.0, 1.0 );
	}
	vec4 getTransmissionSample( const in vec2 fragCoord, const in float roughness, const in float ior ) {
		float lod = log2( transmissionSamplerSize.x ) * applyIorToRoughness( roughness, ior );
		return textureBicubic( transmissionSamplerMap, fragCoord.xy, lod );
	}
	vec3 volumeAttenuation( const in float transmissionDistance, const in vec3 attenuationColor, const in float attenuationDistance ) {
		if ( isinf( attenuationDistance ) ) {
			return vec3( 1.0 );
		} else {
			vec3 attenuationCoefficient = -log( attenuationColor ) / attenuationDistance;
			vec3 transmittance = exp( - attenuationCoefficient * transmissionDistance );			return transmittance;
		}
	}
	vec4 getIBLVolumeRefraction( const in vec3 n, const in vec3 v, const in float roughness, const in vec3 diffuseColor,
		const in vec3 specularColor, const in float specularF90, const in vec3 position, const in mat4 modelMatrix,
		const in mat4 viewMatrix, const in mat4 projMatrix, const in float dispersion, const in float ior, const in float thickness,
		const in vec3 attenuationColor, const in float attenuationDistance ) {
		vec4 transmittedLight;
		vec3 transmittance;
		#ifdef USE_DISPERSION
			float halfSpread = ( ior - 1.0 ) * 0.025 * dispersion;
			vec3 iors = vec3( ior - halfSpread, ior, ior + halfSpread );
			for ( int i = 0; i < 3; i ++ ) {
				vec3 transmissionRay = getVolumeTransmissionRay( n, v, thickness, iors[ i ], modelMatrix );
				vec3 refractedRayExit = position + transmissionRay;
				vec4 ndcPos = projMatrix * viewMatrix * vec4( refractedRayExit, 1.0 );
				vec2 refractionCoords = ndcPos.xy / ndcPos.w;
				refractionCoords += 1.0;
				refractionCoords /= 2.0;
				vec4 transmissionSample = getTransmissionSample( refractionCoords, roughness, iors[ i ] );
				transmittedLight[ i ] = transmissionSample[ i ];
				transmittedLight.a += transmissionSample.a;
				transmittance[ i ] = diffuseColor[ i ] * volumeAttenuation( length( transmissionRay ), attenuationColor, attenuationDistance )[ i ];
			}
			transmittedLight.a /= 3.0;
		#else
			vec3 transmissionRay = getVolumeTransmissionRay( n, v, thickness, ior, modelMatrix );
			vec3 refractedRayExit = position + transmissionRay;
			vec4 ndcPos = projMatrix * viewMatrix * vec4( refractedRayExit, 1.0 );
			vec2 refractionCoords = ndcPos.xy / ndcPos.w;
			refractionCoords += 1.0;
			refractionCoords /= 2.0;
			transmittedLight = getTransmissionSample( refractionCoords, roughness, ior );
			transmittance = diffuseColor * volumeAttenuation( length( transmissionRay ), attenuationColor, attenuationDistance );
		#endif
		vec3 attenuatedColor = transmittance * transmittedLight.rgb;
		vec3 F = EnvironmentBRDF( n, v, specularColor, specularF90, roughness );
		float transmittanceFactor = ( transmittance.r + transmittance.g + transmittance.b ) / 3.0;
		return vec4( ( 1.0 - F ) * attenuatedColor, 1.0 - ( 1.0 - transmittedLight.a ) * transmittanceFactor );
	}
#endif`,N0=`#if defined( USE_UV ) || defined( USE_ANISOTROPY )
	varying vec2 vUv;
#endif
#ifdef USE_MAP
	varying vec2 vMapUv;
#endif
#ifdef USE_ALPHAMAP
	varying vec2 vAlphaMapUv;
#endif
#ifdef USE_LIGHTMAP
	varying vec2 vLightMapUv;
#endif
#ifdef USE_AOMAP
	varying vec2 vAoMapUv;
#endif
#ifdef USE_BUMPMAP
	varying vec2 vBumpMapUv;
#endif
#ifdef USE_NORMALMAP
	varying vec2 vNormalMapUv;
#endif
#ifdef USE_EMISSIVEMAP
	varying vec2 vEmissiveMapUv;
#endif
#ifdef USE_METALNESSMAP
	varying vec2 vMetalnessMapUv;
#endif
#ifdef USE_ROUGHNESSMAP
	varying vec2 vRoughnessMapUv;
#endif
#ifdef USE_DIFFUSE_ROUGHNESSMAP
	varying vec2 vDiffuseRoughnessMapUv;
#endif
#ifdef USE_ANISOTROPYMAP
	varying vec2 vAnisotropyMapUv;
#endif
#ifdef USE_CLEARCOATMAP
	varying vec2 vClearcoatMapUv;
#endif
#ifdef USE_CLEARCOAT_NORMALMAP
	varying vec2 vClearcoatNormalMapUv;
#endif
#ifdef USE_CLEARCOAT_ROUGHNESSMAP
	varying vec2 vClearcoatRoughnessMapUv;
#endif
#ifdef USE_IRIDESCENCEMAP
	varying vec2 vIridescenceMapUv;
#endif
#ifdef USE_IRIDESCENCE_THICKNESSMAP
	varying vec2 vIridescenceThicknessMapUv;
#endif
#ifdef USE_SHEEN_COLORMAP
	varying vec2 vSheenColorMapUv;
#endif
#ifdef USE_SHEEN_ROUGHNESSMAP
	varying vec2 vSheenRoughnessMapUv;
#endif
#ifdef USE_SPECULARMAP
	varying vec2 vSpecularMapUv;
#endif
#ifdef USE_SPECULAR_COLORMAP
	varying vec2 vSpecularColorMapUv;
#endif
#ifdef USE_SPECULAR_INTENSITYMAP
	varying vec2 vSpecularIntensityMapUv;
#endif
#ifdef USE_TRANSMISSIONMAP
	uniform mat3 transmissionMapTransform;
	varying vec2 vTransmissionMapUv;
#endif
#ifdef USE_THICKNESSMAP
	uniform mat3 thicknessMapTransform;
	varying vec2 vThicknessMapUv;
#endif`,U0=`#if defined( USE_UV ) || defined( USE_ANISOTROPY )
	varying vec2 vUv;
#endif
#ifdef USE_MAP
	uniform mat3 mapTransform;
	varying vec2 vMapUv;
#endif
#ifdef USE_ALPHAMAP
	uniform mat3 alphaMapTransform;
	varying vec2 vAlphaMapUv;
#endif
#ifdef USE_LIGHTMAP
	uniform mat3 lightMapTransform;
	varying vec2 vLightMapUv;
#endif
#ifdef USE_AOMAP
	uniform mat3 aoMapTransform;
	varying vec2 vAoMapUv;
#endif
#ifdef USE_BUMPMAP
	uniform mat3 bumpMapTransform;
	varying vec2 vBumpMapUv;
#endif
#ifdef USE_NORMALMAP
	uniform mat3 normalMapTransform;
	varying vec2 vNormalMapUv;
#endif
#ifdef USE_DISPLACEMENTMAP
	uniform mat3 displacementMapTransform;
	varying vec2 vDisplacementMapUv;
#endif
#ifdef USE_EMISSIVEMAP
	uniform mat3 emissiveMapTransform;
	varying vec2 vEmissiveMapUv;
#endif
#ifdef USE_METALNESSMAP
	uniform mat3 metalnessMapTransform;
	varying vec2 vMetalnessMapUv;
#endif
#ifdef USE_ROUGHNESSMAP
	uniform mat3 roughnessMapTransform;
	varying vec2 vRoughnessMapUv;
#endif
#ifdef USE_DIFFUSE_ROUGHNESSMAP
	uniform mat3 diffuseRoughnessMapTransform;
	varying vec2 vDiffuseRoughnessMapUv;
#endif
#ifdef USE_ANISOTROPYMAP
	uniform mat3 anisotropyMapTransform;
	varying vec2 vAnisotropyMapUv;
#endif
#ifdef USE_CLEARCOATMAP
	uniform mat3 clearcoatMapTransform;
	varying vec2 vClearcoatMapUv;
#endif
#ifdef USE_CLEARCOAT_NORMALMAP
	uniform mat3 clearcoatNormalMapTransform;
	varying vec2 vClearcoatNormalMapUv;
#endif
#ifdef USE_CLEARCOAT_ROUGHNESSMAP
	uniform mat3 clearcoatRoughnessMapTransform;
	varying vec2 vClearcoatRoughnessMapUv;
#endif
#ifdef USE_SHEEN_COLORMAP
	uniform mat3 sheenColorMapTransform;
	varying vec2 vSheenColorMapUv;
#endif
#ifdef USE_SHEEN_ROUGHNESSMAP
	uniform mat3 sheenRoughnessMapTransform;
	varying vec2 vSheenRoughnessMapUv;
#endif
#ifdef USE_IRIDESCENCEMAP
	uniform mat3 iridescenceMapTransform;
	varying vec2 vIridescenceMapUv;
#endif
#ifdef USE_IRIDESCENCE_THICKNESSMAP
	uniform mat3 iridescenceThicknessMapTransform;
	varying vec2 vIridescenceThicknessMapUv;
#endif
#ifdef USE_SPECULARMAP
	uniform mat3 specularMapTransform;
	varying vec2 vSpecularMapUv;
#endif
#ifdef USE_SPECULAR_COLORMAP
	uniform mat3 specularColorMapTransform;
	varying vec2 vSpecularColorMapUv;
#endif
#ifdef USE_SPECULAR_INTENSITYMAP
	uniform mat3 specularIntensityMapTransform;
	varying vec2 vSpecularIntensityMapUv;
#endif
#ifdef USE_TRANSMISSIONMAP
	uniform mat3 transmissionMapTransform;
	varying vec2 vTransmissionMapUv;
#endif
#ifdef USE_THICKNESSMAP
	uniform mat3 thicknessMapTransform;
	varying vec2 vThicknessMapUv;
#endif`,F0=`#if defined( USE_UV ) || defined( USE_ANISOTROPY )
	vUv = vec3( uv, 1 ).xy;
#endif
#ifdef USE_MAP
	vMapUv = ( mapTransform * vec3( MAP_UV, 1 ) ).xy;
#endif
#ifdef USE_ALPHAMAP
	vAlphaMapUv = ( alphaMapTransform * vec3( ALPHAMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_LIGHTMAP
	vLightMapUv = ( lightMapTransform * vec3( LIGHTMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_AOMAP
	vAoMapUv = ( aoMapTransform * vec3( AOMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_BUMPMAP
	vBumpMapUv = ( bumpMapTransform * vec3( BUMPMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_NORMALMAP
	vNormalMapUv = ( normalMapTransform * vec3( NORMALMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_DISPLACEMENTMAP
	vDisplacementMapUv = ( displacementMapTransform * vec3( DISPLACEMENTMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_EMISSIVEMAP
	vEmissiveMapUv = ( emissiveMapTransform * vec3( EMISSIVEMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_METALNESSMAP
	vMetalnessMapUv = ( metalnessMapTransform * vec3( METALNESSMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_ROUGHNESSMAP
	vRoughnessMapUv = ( roughnessMapTransform * vec3( ROUGHNESSMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_DIFFUSE_ROUGHNESSMAP
	vDiffuseRoughnessMapUv = ( diffuseRoughnessMapTransform * vec3( DIFFUSE_ROUGHNESSMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_ANISOTROPYMAP
	vAnisotropyMapUv = ( anisotropyMapTransform * vec3( ANISOTROPYMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_CLEARCOATMAP
	vClearcoatMapUv = ( clearcoatMapTransform * vec3( CLEARCOATMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_CLEARCOAT_NORMALMAP
	vClearcoatNormalMapUv = ( clearcoatNormalMapTransform * vec3( CLEARCOAT_NORMALMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_CLEARCOAT_ROUGHNESSMAP
	vClearcoatRoughnessMapUv = ( clearcoatRoughnessMapTransform * vec3( CLEARCOAT_ROUGHNESSMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_IRIDESCENCEMAP
	vIridescenceMapUv = ( iridescenceMapTransform * vec3( IRIDESCENCEMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_IRIDESCENCE_THICKNESSMAP
	vIridescenceThicknessMapUv = ( iridescenceThicknessMapTransform * vec3( IRIDESCENCE_THICKNESSMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_SHEEN_COLORMAP
	vSheenColorMapUv = ( sheenColorMapTransform * vec3( SHEEN_COLORMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_SHEEN_ROUGHNESSMAP
	vSheenRoughnessMapUv = ( sheenRoughnessMapTransform * vec3( SHEEN_ROUGHNESSMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_SPECULARMAP
	vSpecularMapUv = ( specularMapTransform * vec3( SPECULARMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_SPECULAR_COLORMAP
	vSpecularColorMapUv = ( specularColorMapTransform * vec3( SPECULAR_COLORMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_SPECULAR_INTENSITYMAP
	vSpecularIntensityMapUv = ( specularIntensityMapTransform * vec3( SPECULAR_INTENSITYMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_TRANSMISSIONMAP
	vTransmissionMapUv = ( transmissionMapTransform * vec3( TRANSMISSIONMAP_UV, 1 ) ).xy;
#endif
#ifdef USE_THICKNESSMAP
	vThicknessMapUv = ( thicknessMapTransform * vec3( THICKNESSMAP_UV, 1 ) ).xy;
#endif`,O0=`#if defined( USE_ENVMAP ) || defined( DISTANCE ) || defined ( USE_SHADOWMAP ) || defined ( USE_TRANSMISSION ) || NUM_SPOT_LIGHT_COORDS > 0
	vec4 worldPosition = vec4( transformed, 1.0 );
	#ifdef USE_BATCHING
		worldPosition = batchingMatrix * worldPosition;
	#endif
	#ifdef USE_INSTANCING
		worldPosition = instanceMatrix * worldPosition;
	#endif
	worldPosition = modelMatrix * worldPosition;
#endif`,B0=`varying vec2 vUv;
uniform mat3 uvTransform;
void main() {
	vUv = ( uvTransform * vec3( uv, 1 ) ).xy;
	gl_Position = vec4( position.xy, 1.0, 1.0 );
}`,z0=`uniform sampler2D t2D;
uniform float backgroundIntensity;
varying vec2 vUv;
void main() {
	vec4 texColor = texture2D( t2D, vUv );
	#ifdef DECODE_VIDEO_TEXTURE
		texColor = vec4( mix( pow( texColor.rgb * 0.9478672986 + vec3( 0.0521327014 ), vec3( 2.4 ) ), texColor.rgb * 0.0773993808, vec3( lessThanEqual( texColor.rgb, vec3( 0.04045 ) ) ) ), texColor.w );
	#endif
	texColor.rgb *= backgroundIntensity;
	gl_FragColor = texColor;
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
}`,k0=`varying vec3 vWorldDirection;
#include <common>
void main() {
	vWorldDirection = transformDirection( position, modelMatrix );
	#include <begin_vertex>
	#include <project_vertex>
	gl_Position.z = gl_Position.w;
}`,V0=`uniform float backgroundBlurriness;
uniform float backgroundIntensity;
uniform mat3 backgroundRotation;
varying vec3 vWorldDirection;
#include <envmap_common_pars_fragment>
void main() {
	#ifdef ENVMAP_TYPE_PMREM
		vec4 texColor = textureLod( envMap, backgroundRotation * vWorldDirection, roughnessToMip( backgroundBlurriness ) );
	#else
		vec4 texColor = textureCube( envMap, backgroundRotation * vWorldDirection );
	#endif
	texColor.rgb *= backgroundIntensity;
	gl_FragColor = texColor;
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
}`,G0=`varying vec3 vWorldDirection;
#include <common>
void main() {
	vWorldDirection = transformDirection( position, modelMatrix );
	#include <begin_vertex>
	#include <project_vertex>
	gl_Position.z = gl_Position.w;
}`,H0=`uniform samplerCube tCube;
uniform float tFlip;
uniform float opacity;
varying vec3 vWorldDirection;
void main() {
	vec4 texColor = textureCube( tCube, vec3( tFlip * vWorldDirection.x, vWorldDirection.yz ) );
	gl_FragColor = texColor;
	gl_FragColor.a *= opacity;
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
}`,W0=`#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
varying vec2 vHighPrecisionZW;
void main() {
	#include <uv_vertex>
	#include <batching_vertex>
	#include <skinbase_vertex>
	#include <morphinstance_vertex>
	#ifdef USE_DISPLACEMENTMAP
		#include <beginnormal_vertex>
		#include <morphnormal_vertex>
		#include <skinnormal_vertex>
	#endif
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	vHighPrecisionZW = gl_Position.zw;
}`,X0=`#if DEPTH_PACKING == 3200
	uniform float opacity;
#endif
#include <common>
#include <packing>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
varying vec2 vHighPrecisionZW;
void main() {
	vec4 diffuseColor = vec4( 1.0 );
	#include <clipping_planes_fragment>
	#if DEPTH_PACKING == 3200
		diffuseColor.a = opacity;
	#endif
	#include <map_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <logdepthbuf_fragment>
	#ifdef USE_REVERSED_DEPTH_BUFFER
		float fragCoordZ = vHighPrecisionZW[ 0 ] / vHighPrecisionZW[ 1 ];
	#else
		float fragCoordZ = 0.5 * vHighPrecisionZW[ 0 ] / vHighPrecisionZW[ 1 ] + 0.5;
	#endif
	#if DEPTH_PACKING == 3200
		gl_FragColor = vec4( vec3( 1.0 - fragCoordZ ), opacity );
	#elif DEPTH_PACKING == 3201
		gl_FragColor = packDepthToRGBA( fragCoordZ );
	#elif DEPTH_PACKING == 3202
		gl_FragColor = vec4( packDepthToRGB( fragCoordZ ), 1.0 );
	#elif DEPTH_PACKING == 3203
		gl_FragColor = vec4( packDepthToRG( fragCoordZ ), 0.0, 1.0 );
	#endif
}`,q0=`#define DISTANCE
varying vec3 vWorldPosition;
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <batching_vertex>
	#include <skinbase_vertex>
	#include <morphinstance_vertex>
	#ifdef USE_DISPLACEMENTMAP
		#include <beginnormal_vertex>
		#include <morphnormal_vertex>
		#include <skinnormal_vertex>
	#endif
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <worldpos_vertex>
	#include <clipping_planes_vertex>
	vWorldPosition = worldPosition.xyz;
}`,Y0=`#define DISTANCE
uniform vec3 referencePosition;
uniform float nearDistance;
uniform float farDistance;
varying vec3 vWorldPosition;
#include <common>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( 1.0 );
	#include <clipping_planes_fragment>
	#include <map_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	float dist = length( vWorldPosition - referencePosition );
	dist = ( dist - nearDistance ) / ( farDistance - nearDistance );
	dist = saturate( dist );
	gl_FragColor = vec4( dist, 0.0, 0.0, 1.0 );
}`,Z0=`varying vec3 vWorldDirection;
#include <common>
void main() {
	vWorldDirection = transformDirection( position, modelMatrix );
	#include <begin_vertex>
	#include <project_vertex>
}`,J0=`uniform sampler2D tEquirect;
varying vec3 vWorldDirection;
#include <common>
void main() {
	vec3 direction = normalize( vWorldDirection );
	vec2 sampleUV = equirectUv( direction );
	gl_FragColor = texture2D( tEquirect, sampleUV );
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
}`,$0=`uniform float scale;
attribute float lineDistance;
varying float vLineDistance;
#include <common>
#include <uv_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <morphtarget_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	vLineDistance = scale * lineDistance;
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <fog_vertex>
}`,K0=`uniform vec3 diffuse;
uniform float opacity;
uniform float dashSize;
uniform float totalSize;
varying float vLineDistance;
#include <common>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <fog_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	if ( mod( vLineDistance, totalSize ) > dashSize ) {
		discard;
	}
	vec3 outgoingLight = vec3( 0.0 );
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	outgoingLight = diffuseColor.rgb;
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
}`,j0=`#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <envmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#if defined ( USE_ENVMAP ) || defined ( USE_SKINNING )
		#include <beginnormal_vertex>
		#include <morphnormal_vertex>
		#include <skinbase_vertex>
		#include <skinnormal_vertex>
		#include <defaultnormal_vertex>
	#endif
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <worldpos_vertex>
	#include <envmap_vertex>
	#include <fog_vertex>
}`,Q0=`uniform vec3 diffuse;
uniform float opacity;
#ifndef FLAT_SHADED
	varying vec3 vNormal;
#endif
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_pars_fragment>
#include <fog_pars_fragment>
#include <specularmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <specularmap_fragment>
	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	#ifdef USE_LIGHTMAP
		vec4 lightMapTexel = texture2D( lightMap, vLightMapUv );
		reflectedLight.indirectDiffuse += lightMapTexel.rgb * lightMapIntensity * RECIPROCAL_PI;
	#else
		reflectedLight.indirectDiffuse += vec3( 1.0 );
	#endif
	#include <aomap_fragment>
	reflectedLight.indirectDiffuse *= diffuseColor.rgb;
	vec3 outgoingLight = reflectedLight.indirectDiffuse;
	#include <envmap_fragment>
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}`,e_=`#define LAMBERT
varying vec3 vViewPosition;
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <envmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	vViewPosition = - mvPosition.xyz;
	#include <worldpos_vertex>
	#include <envmap_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>
}`,t_=`#define LAMBERT
uniform vec3 diffuse;
uniform vec3 emissive;
uniform float opacity;
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_pars_fragment>
#include <envmap_physical_pars_fragment>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_lambert_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <specularmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <specularmap_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <emissivemap_fragment>
	#include <lights_lambert_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>
	#include <aomap_fragment>
	vec3 outgoingLight = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse + totalEmissiveRadiance;
	#include <envmap_fragment>
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}`,n_=`#define MATCAP
varying vec3 vViewPosition;
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <color_pars_vertex>
#include <displacementmap_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <fog_vertex>
	vViewPosition = - mvPosition.xyz;
}`,i_=`#define MATCAP
uniform vec3 diffuse;
uniform float opacity;
uniform sampler2D matcap;
varying vec3 vViewPosition;
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <fog_pars_fragment>
#include <normal_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	vec3 viewDir = normalize( vViewPosition );
	vec3 x = normalize( vec3( viewDir.z, 0.0, - viewDir.x ) );
	vec3 y = cross( viewDir, x );
	vec2 uv = vec2( dot( x, normal ), dot( y, normal ) ) * 0.495 + 0.5;
	#ifdef USE_MATCAP
		vec4 matcapColor = texture2D( matcap, uv );
	#else
		vec4 matcapColor = vec4( vec3( mix( 0.2, 0.8, uv.y ) ), 1.0 );
	#endif
	vec3 outgoingLight = diffuseColor.rgb * matcapColor.rgb;
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}`,s_=`#define NORMAL
#if defined( FLAT_SHADED ) || defined( USE_BUMPMAP ) || defined( USE_NORMALMAP_TANGENTSPACE )
	varying vec3 vViewPosition;
#endif
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphinstance_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
#if defined( FLAT_SHADED ) || defined( USE_BUMPMAP ) || defined( USE_NORMALMAP_TANGENTSPACE )
	vViewPosition = - mvPosition.xyz;
#endif
}`,r_=`#define NORMAL
uniform float opacity;
#if defined( FLAT_SHADED ) || defined( USE_BUMPMAP ) || defined( USE_NORMALMAP_TANGENTSPACE )
	varying vec3 vViewPosition;
#endif
#include <uv_pars_fragment>
#include <normal_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( 0.0, 0.0, 0.0, opacity );
	#include <clipping_planes_fragment>
	#include <logdepthbuf_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	gl_FragColor = vec4( normalize( normal ) * 0.5 + 0.5, diffuseColor.a );
	#ifdef OPAQUE
		gl_FragColor.a = 1.0;
	#endif
}`,o_=`#define PHONG
varying vec3 vViewPosition;
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <envmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphinstance_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	vViewPosition = - mvPosition.xyz;
	#include <worldpos_vertex>
	#include <envmap_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>
}`,a_=`#define PHONG
uniform vec3 diffuse;
uniform vec3 emissive;
uniform vec3 specular;
uniform float shininess;
uniform float opacity;
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_pars_fragment>
#include <envmap_physical_pars_fragment>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_phong_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <specularmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <specularmap_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <emissivemap_fragment>
	#include <lights_phong_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>
	#include <aomap_fragment>
	vec3 outgoingLight = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse + reflectedLight.directSpecular + reflectedLight.indirectSpecular + totalEmissiveRadiance;
	#include <envmap_fragment>
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}`,l_=`#define STANDARD
varying vec3 vViewPosition;
#ifdef USE_TRANSMISSION
	varying vec3 vWorldPosition;
#endif
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	vViewPosition = - mvPosition.xyz;
	#include <worldpos_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>
#ifdef USE_TRANSMISSION
	vWorldPosition = worldPosition.xyz;
#endif
}`,c_=`#define STANDARD
#ifdef PHYSICAL
	#define IOR
	#define USE_SPECULAR
#endif
uniform vec3 diffuse;
uniform vec3 emissive;
uniform float roughness;
uniform float metalness;
uniform float opacity;
#ifdef IOR
	uniform float ior;
#endif
#ifdef USE_SPECULAR
	uniform float specularIntensity;
	uniform vec3 specularColor;
	#ifdef USE_SPECULAR_COLORMAP
		uniform sampler2D specularColorMap;
	#endif
	#ifdef USE_SPECULAR_INTENSITYMAP
		uniform sampler2D specularIntensityMap;
	#endif
#endif
#ifdef USE_CLEARCOAT
	uniform float clearcoat;
	uniform float clearcoatRoughness;
#endif
#ifdef USE_DIFFUSE_ROUGHNESS
	uniform float diffuseRoughness;
	#ifdef USE_DIFFUSE_ROUGHNESSMAP
		uniform sampler2D diffuseRoughnessMap;
	#endif
#endif
#ifdef USE_DISPERSION
	uniform float dispersion;
#endif
#ifdef USE_RETROREFLECTION
	uniform float retroreflectivity;
#endif
#ifdef USE_IRIDESCENCE
	uniform float iridescence;
	uniform float iridescenceIOR;
	uniform float iridescenceThicknessMinimum;
	uniform float iridescenceThicknessMaximum;
#endif
#ifdef USE_SHEEN
	uniform vec3 sheenColor;
	uniform float sheenRoughness;
	#ifdef USE_SHEEN_COLORMAP
		uniform sampler2D sheenColorMap;
	#endif
	#ifdef USE_SHEEN_ROUGHNESSMAP
		uniform sampler2D sheenRoughnessMap;
	#endif
#endif
#ifdef USE_ANISOTROPY
	uniform vec2 anisotropyVector;
	#ifdef USE_ANISOTROPYMAP
		uniform sampler2D anisotropyMap;
	#endif
#endif
varying vec3 vViewPosition;
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <iridescence_fragment>
#include <envmap_common_pars_fragment>
#include <envmap_physical_pars_fragment>
#include <fog_pars_fragment>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_physical_pars_fragment>
#include <transmission_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <clearcoat_pars_fragment>
#include <iridescence_pars_fragment>
#include <roughnessmap_pars_fragment>
#include <metalnessmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <roughnessmap_fragment>
	#include <metalnessmap_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <clearcoat_normal_fragment_begin>
	#include <clearcoat_normal_fragment_maps>
	#include <emissivemap_fragment>
	#include <lights_physical_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>
	#include <aomap_fragment>
	vec3 totalDiffuse = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse;
	vec3 totalSpecular = reflectedLight.directSpecular + reflectedLight.indirectSpecular;
	#include <transmission_fragment>
	vec3 outgoingLight = totalDiffuse + totalSpecular + totalEmissiveRadiance;
	#ifdef USE_SHEEN
 
		outgoingLight = outgoingLight + sheenSpecularDirect + sheenSpecularIndirect;
 
 	#endif
	#ifdef USE_CLEARCOAT
		float dotNVcc = saturate( dot( geometryClearcoatNormal, geometryViewDir ) );
		vec3 Fcc = F_Schlick( material.clearcoatF0, material.clearcoatF90, dotNVcc );
		outgoingLight = outgoingLight * ( 1.0 - material.clearcoat * Fcc ) + ( clearcoatSpecularDirect + clearcoatSpecularIndirect ) * material.clearcoat;
	#endif
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}`,h_=`#define TOON
varying vec3 vViewPosition;
#include <common>
#include <batching_pars_vertex>
#include <uv_pars_vertex>
#include <displacementmap_pars_vertex>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <normal_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <shadowmap_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <normal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <displacementmap_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	vViewPosition = - mvPosition.xyz;
	#include <worldpos_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>
}`,u_=`#define TOON
uniform vec3 diffuse;
uniform vec3 emissive;
uniform float opacity;
#include <common>
#include <dithering_pars_fragment>
#include <color_pars_fragment>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <aomap_pars_fragment>
#include <lightmap_pars_fragment>
#include <emissivemap_pars_fragment>
#include <gradientmap_pars_fragment>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <normal_pars_fragment>
#include <lights_toon_pars_fragment>
#include <shadowmap_pars_fragment>
#include <bumpmap_pars_fragment>
#include <normalmap_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	ReflectedLight reflectedLight = ReflectedLight( vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ), vec3( 0.0 ) );
	vec3 totalEmissiveRadiance = emissive;
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <color_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	#include <normal_fragment_begin>
	#include <normal_fragment_maps>
	#include <emissivemap_fragment>
	#include <lights_toon_fragment>
	#include <lights_fragment_begin>
	#include <lights_fragment_maps>
	#include <lights_fragment_end>
	#include <aomap_fragment>
	vec3 outgoingLight = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse + totalEmissiveRadiance;
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
	#include <dithering_fragment>
}`,f_=`uniform float size;
uniform float scale;
#include <common>
#include <color_pars_vertex>
#include <fog_pars_vertex>
#include <morphtarget_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
#ifdef USE_POINTS_UV
	varying vec2 vUv;
	uniform mat3 uvTransform;
#endif
void main() {
	#ifdef USE_POINTS_UV
		vUv = ( uvTransform * vec3( uv, 1 ) ).xy;
	#endif
	#include <color_vertex>
	#include <morphinstance_vertex>
	#include <morphcolor_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <project_vertex>
	gl_PointSize = size;
	#ifdef USE_SIZEATTENUATION
		bool isPerspective = isPerspectiveMatrix( projectionMatrix );
		if ( isPerspective ) gl_PointSize *= ( scale / - mvPosition.z );
	#endif
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <worldpos_vertex>
	#include <fog_vertex>
}`,d_=`uniform vec3 diffuse;
uniform float opacity;
#include <common>
#include <color_pars_fragment>
#include <map_particle_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <fog_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	vec3 outgoingLight = vec3( 0.0 );
	#include <logdepthbuf_fragment>
	#include <map_particle_fragment>
	#include <color_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	outgoingLight = diffuseColor.rgb;
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
}`,p_=`#include <common>
#include <batching_pars_vertex>
#include <fog_pars_vertex>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <shadowmap_pars_vertex>
void main() {
	#include <batching_vertex>
	#include <beginnormal_vertex>
	#include <morphinstance_vertex>
	#include <morphnormal_vertex>
	#include <skinbase_vertex>
	#include <skinnormal_vertex>
	#include <defaultnormal_vertex>
	#include <begin_vertex>
	#include <morphtarget_vertex>
	#include <skinning_vertex>
	#include <project_vertex>
	#include <logdepthbuf_vertex>
	#include <worldpos_vertex>
	#include <shadowmap_vertex>
	#include <fog_vertex>
}`,m_=`uniform vec3 color;
uniform float opacity;
#include <common>
#include <fog_pars_fragment>
#include <bsdfs>
#include <lights_pars_begin>
#include <logdepthbuf_pars_fragment>
#include <shadowmap_pars_fragment>
#include <shadowmask_pars_fragment>
void main() {
	#include <logdepthbuf_fragment>
	gl_FragColor = vec4( color, opacity * ( 1.0 - getShadowMask() ) );
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
	#include <premultiplied_alpha_fragment>
}`,g_=`uniform float rotation;
uniform vec2 center;
#include <common>
#include <uv_pars_vertex>
#include <fog_pars_vertex>
#include <logdepthbuf_pars_vertex>
#include <clipping_planes_pars_vertex>
void main() {
	#include <uv_vertex>
	vec4 mvPosition = modelViewMatrix[ 3 ];
	vec2 scale = vec2( length( modelMatrix[ 0 ].xyz ), length( modelMatrix[ 1 ].xyz ) );
	#ifndef USE_SIZEATTENUATION
		bool isPerspective = isPerspectiveMatrix( projectionMatrix );
		if ( isPerspective ) scale *= - mvPosition.z;
	#endif
	vec2 alignedPosition = ( position.xy - ( center - vec2( 0.5 ) ) ) * scale;
	vec2 rotatedPosition;
	rotatedPosition.x = cos( rotation ) * alignedPosition.x - sin( rotation ) * alignedPosition.y;
	rotatedPosition.y = sin( rotation ) * alignedPosition.x + cos( rotation ) * alignedPosition.y;
	mvPosition.xy += rotatedPosition;
	gl_Position = projectionMatrix * mvPosition;
	#include <logdepthbuf_vertex>
	#include <clipping_planes_vertex>
	#include <fog_vertex>
}`,__=`uniform vec3 diffuse;
uniform float opacity;
#include <common>
#include <uv_pars_fragment>
#include <map_pars_fragment>
#include <alphamap_pars_fragment>
#include <alphatest_pars_fragment>
#include <alphahash_pars_fragment>
#include <fog_pars_fragment>
#include <logdepthbuf_pars_fragment>
#include <clipping_planes_pars_fragment>
void main() {
	vec4 diffuseColor = vec4( diffuse, opacity );
	#include <clipping_planes_fragment>
	vec3 outgoingLight = vec3( 0.0 );
	#include <logdepthbuf_fragment>
	#include <map_fragment>
	#include <alphamap_fragment>
	#include <alphatest_fragment>
	#include <alphahash_fragment>
	outgoingLight = diffuseColor.rgb;
	#include <opaque_fragment>
	#include <tonemapping_fragment>
	#include <colorspace_fragment>
	#include <fog_fragment>
}`,nt={alphahash_fragment:zm,alphahash_pars_fragment:km,alphamap_fragment:Vm,alphamap_pars_fragment:Gm,alphatest_fragment:Hm,alphatest_pars_fragment:Wm,aomap_fragment:Xm,aomap_pars_fragment:qm,batching_pars_vertex:Ym,batching_vertex:Zm,begin_vertex:Jm,beginnormal_vertex:$m,bsdfs:Km,iridescence_fragment:jm,bumpmap_pars_fragment:Qm,clipping_planes_fragment:eg,clipping_planes_pars_fragment:tg,clipping_planes_pars_vertex:ng,clipping_planes_vertex:ig,color_fragment:sg,color_pars_fragment:rg,color_pars_vertex:og,color_vertex:ag,common:lg,defaultnormal_vertex:cg,displacementmap_pars_vertex:hg,displacementmap_vertex:ug,emissivemap_fragment:fg,emissivemap_pars_fragment:dg,colorspace_fragment:pg,colorspace_pars_fragment:mg,envmap_fragment:gg,envmap_common_pars_fragment:_g,envmap_pars_fragment:xg,envmap_pars_vertex:yg,envmap_physical_pars_fragment:Pg,envmap_vertex:vg,fog_vertex:Sg,fog_pars_vertex:Mg,fog_fragment:bg,fog_pars_fragment:wg,gradientmap_pars_fragment:Ag,lightmap_pars_fragment:Tg,lights_lambert_fragment:Eg,lights_lambert_pars_fragment:Rg,lights_pars_begin:Cg,lights_toon_fragment:Ig,lights_toon_pars_fragment:Dg,lights_phong_fragment:Lg,lights_phong_pars_fragment:Ng,lights_physical_fragment:Ug,lights_physical_pars_fragment:Fg,lights_fragment_begin:Og,lights_fragment_maps:Bg,lights_fragment_end:zg,lightprobes_pars_fragment:kg,logdepthbuf_fragment:Vg,logdepthbuf_pars_fragment:Gg,logdepthbuf_pars_vertex:Hg,logdepthbuf_vertex:Wg,map_fragment:Xg,map_pars_fragment:qg,map_particle_fragment:Yg,map_particle_pars_fragment:Zg,metalnessmap_fragment:Jg,metalnessmap_pars_fragment:$g,morphinstance_vertex:Kg,morphcolor_vertex:jg,morphnormal_vertex:Qg,morphtarget_pars_vertex:e0,morphtarget_vertex:t0,normal_fragment_begin:n0,normal_fragment_maps:i0,normal_pars_fragment:s0,normal_pars_vertex:r0,normal_vertex:o0,normalmap_pars_fragment:a0,clearcoat_normal_fragment_begin:l0,clearcoat_normal_fragment_maps:c0,clearcoat_pars_fragment:h0,iridescence_pars_fragment:u0,opaque_fragment:f0,packing:d0,premultiplied_alpha_fragment:p0,project_vertex:m0,dithering_fragment:g0,dithering_pars_fragment:_0,roughnessmap_fragment:x0,roughnessmap_pars_fragment:y0,shadowmap_pars_fragment:v0,shadowmap_pars_vertex:S0,shadowmap_vertex:M0,shadowmask_pars_fragment:b0,skinbase_vertex:w0,skinning_pars_vertex:A0,skinning_vertex:T0,skinnormal_vertex:E0,specularmap_fragment:R0,specularmap_pars_fragment:C0,tonemapping_fragment:P0,tonemapping_pars_fragment:I0,transmission_fragment:D0,transmission_pars_fragment:L0,uv_pars_fragment:N0,uv_pars_vertex:U0,uv_vertex:F0,worldpos_vertex:O0,background_vert:B0,background_frag:z0,backgroundCube_vert:k0,backgroundCube_frag:V0,cube_vert:G0,cube_frag:H0,depth_vert:W0,depth_frag:X0,distance_vert:q0,distance_frag:Y0,equirect_vert:Z0,equirect_frag:J0,linedashed_vert:$0,linedashed_frag:K0,meshbasic_vert:j0,meshbasic_frag:Q0,meshlambert_vert:e_,meshlambert_frag:t_,meshmatcap_vert:n_,meshmatcap_frag:i_,meshnormal_vert:s_,meshnormal_frag:r_,meshphong_vert:o_,meshphong_frag:a_,meshphysical_vert:l_,meshphysical_frag:c_,meshtoon_vert:h_,meshtoon_frag:u_,points_vert:f_,points_frag:d_,shadow_vert:p_,shadow_frag:m_,sprite_vert:g_,sprite_frag:__},Ie={common:{diffuse:{value:new Ke(16777215)},opacity:{value:1},map:{value:null},mapTransform:{value:new Ze},alphaMap:{value:null},alphaMapTransform:{value:new Ze},alphaTest:{value:0}},specularmap:{specularMap:{value:null},specularMapTransform:{value:new Ze}},envmap:{envMap:{value:null},envMapRotation:{value:new Ze},reflectivity:{value:1},ior:{value:1.5},refractionRatio:{value:.98},dfgLUT:{value:null}},aomap:{aoMap:{value:null},aoMapIntensity:{value:1},aoMapTransform:{value:new Ze}},lightmap:{lightMap:{value:null},lightMapIntensity:{value:1},lightMapTransform:{value:new Ze}},bumpmap:{bumpMap:{value:null},bumpMapTransform:{value:new Ze},bumpScale:{value:1}},normalmap:{normalMap:{value:null},normalMapTransform:{value:new Ze},normalScale:{value:new pe(1,1)}},displacementmap:{displacementMap:{value:null},displacementMapTransform:{value:new Ze},displacementScale:{value:1},displacementBias:{value:0}},emissivemap:{emissiveMap:{value:null},emissiveMapTransform:{value:new Ze}},metalnessmap:{metalnessMap:{value:null},metalnessMapTransform:{value:new Ze}},roughnessmap:{roughnessMap:{value:null},roughnessMapTransform:{value:new Ze}},gradientmap:{gradientMap:{value:null}},fog:{fogDensity:{value:25e-5},fogNear:{value:1},fogFar:{value:2e3},fogColor:{value:new Ke(16777215)}},lights:{ambientLightColor:{value:[]},lightProbe:{value:[]},sunLights:{value:[],properties:{direction:{},color:{}}},sunLightShadows:{value:[],properties:{shadowIntensity:1,shadowBias:{},shadowNormalBias:{},shadowRadius:{},shadowMapSize:{}}},sunShadowMatrix:{value:[]},sunShadowCascade:{value:[]},directionalLights:{value:[],properties:{direction:{},color:{}}},directionalLightShadows:{value:[],properties:{shadowIntensity:1,shadowBias:{},shadowNormalBias:{},shadowRadius:{},shadowMapSize:{}}},directionalShadowMatrix:{value:[]},spotLights:{value:[],properties:{color:{},position:{},direction:{},distance:{},coneCos:{},penumbraCos:{},decay:{}}},spotLightShadows:{value:[],properties:{shadowIntensity:1,shadowBias:{},shadowNormalBias:{},shadowRadius:{},shadowMapSize:{}}},spotLightMap:{value:[]},spotLightMatrix:{value:[]},pointLights:{value:[],properties:{color:{},position:{},decay:{},distance:{}}},pointLightShadows:{value:[],properties:{shadowIntensity:1,shadowBias:{},shadowNormalBias:{},shadowRadius:{},shadowMapSize:{},shadowCameraNear:{},shadowCameraFar:{}}},pointShadowMatrix:{value:[]},hemisphereLights:{value:[],properties:{direction:{},skyColor:{},groundColor:{}}},rectAreaLights:{value:[],properties:{color:{},position:{},width:{},height:{}}},ltc_1:{value:null},ltc_2:{value:null},probesSH:{value:[]},probesMin:{value:[]},probesMax:{value:[]},probesResolution:{value:[]}},points:{diffuse:{value:new Ke(16777215)},opacity:{value:1},size:{value:1},scale:{value:1},map:{value:null},alphaMap:{value:null},alphaMapTransform:{value:new Ze},alphaTest:{value:0},uvTransform:{value:new Ze}},sprite:{diffuse:{value:new Ke(16777215)},opacity:{value:1},center:{value:new pe(.5,.5)},rotation:{value:0},map:{value:null},mapTransform:{value:new Ze},alphaMap:{value:null},alphaMapTransform:{value:new Ze},alphaTest:{value:0}}},Xn={basic:{uniforms:jt([Ie.common,Ie.specularmap,Ie.envmap,Ie.aomap,Ie.lightmap,Ie.fog]),vertexShader:nt.meshbasic_vert,fragmentShader:nt.meshbasic_frag},lambert:{uniforms:jt([Ie.common,Ie.specularmap,Ie.envmap,Ie.aomap,Ie.lightmap,Ie.emissivemap,Ie.bumpmap,Ie.normalmap,Ie.displacementmap,Ie.fog,Ie.lights,{emissive:{value:new Ke(0)},envMapIntensity:{value:1}}]),vertexShader:nt.meshlambert_vert,fragmentShader:nt.meshlambert_frag},phong:{uniforms:jt([Ie.common,Ie.specularmap,Ie.envmap,Ie.aomap,Ie.lightmap,Ie.emissivemap,Ie.bumpmap,Ie.normalmap,Ie.displacementmap,Ie.fog,Ie.lights,{emissive:{value:new Ke(0)},specular:{value:new Ke(1118481)},shininess:{value:30},envMapIntensity:{value:1}}]),vertexShader:nt.meshphong_vert,fragmentShader:nt.meshphong_frag},standard:{uniforms:jt([Ie.common,Ie.envmap,Ie.aomap,Ie.lightmap,Ie.emissivemap,Ie.bumpmap,Ie.normalmap,Ie.displacementmap,Ie.roughnessmap,Ie.metalnessmap,Ie.fog,Ie.lights,{emissive:{value:new Ke(0)},roughness:{value:1},metalness:{value:0},envMapIntensity:{value:1}}]),vertexShader:nt.meshphysical_vert,fragmentShader:nt.meshphysical_frag},toon:{uniforms:jt([Ie.common,Ie.aomap,Ie.lightmap,Ie.emissivemap,Ie.bumpmap,Ie.normalmap,Ie.displacementmap,Ie.gradientmap,Ie.fog,Ie.lights,{emissive:{value:new Ke(0)}}]),vertexShader:nt.meshtoon_vert,fragmentShader:nt.meshtoon_frag},matcap:{uniforms:jt([Ie.common,Ie.bumpmap,Ie.normalmap,Ie.displacementmap,Ie.fog,{matcap:{value:null}}]),vertexShader:nt.meshmatcap_vert,fragmentShader:nt.meshmatcap_frag},points:{uniforms:jt([Ie.points,Ie.fog]),vertexShader:nt.points_vert,fragmentShader:nt.points_frag},dashed:{uniforms:jt([Ie.common,Ie.fog,{scale:{value:1},dashSize:{value:1},totalSize:{value:2}}]),vertexShader:nt.linedashed_vert,fragmentShader:nt.linedashed_frag},depth:{uniforms:jt([Ie.common,Ie.displacementmap]),vertexShader:nt.depth_vert,fragmentShader:nt.depth_frag},normal:{uniforms:jt([Ie.common,Ie.bumpmap,Ie.normalmap,Ie.displacementmap,{opacity:{value:1}}]),vertexShader:nt.meshnormal_vert,fragmentShader:nt.meshnormal_frag},sprite:{uniforms:jt([Ie.sprite,Ie.fog]),vertexShader:nt.sprite_vert,fragmentShader:nt.sprite_frag},background:{uniforms:{uvTransform:{value:new Ze},t2D:{value:null},backgroundIntensity:{value:1}},vertexShader:nt.background_vert,fragmentShader:nt.background_frag},backgroundCube:{uniforms:{envMap:{value:null},backgroundBlurriness:{value:0},backgroundIntensity:{value:1},backgroundRotation:{value:new Ze}},vertexShader:nt.backgroundCube_vert,fragmentShader:nt.backgroundCube_frag},cube:{uniforms:{tCube:{value:null},tFlip:{value:-1},opacity:{value:1}},vertexShader:nt.cube_vert,fragmentShader:nt.cube_frag},equirect:{uniforms:{tEquirect:{value:null}},vertexShader:nt.equirect_vert,fragmentShader:nt.equirect_frag},distance:{uniforms:jt([Ie.common,Ie.displacementmap,{referencePosition:{value:new L},nearDistance:{value:1},farDistance:{value:1e3}}]),vertexShader:nt.distance_vert,fragmentShader:nt.distance_frag},shadow:{uniforms:jt([Ie.lights,Ie.fog,{color:{value:new Ke(0)},opacity:{value:1}}]),vertexShader:nt.shadow_vert,fragmentShader:nt.shadow_frag}};Xn.physical={uniforms:jt([Xn.standard.uniforms,{clearcoat:{value:0},clearcoatMap:{value:null},clearcoatMapTransform:{value:new Ze},clearcoatNormalMap:{value:null},clearcoatNormalMapTransform:{value:new Ze},clearcoatNormalScale:{value:new pe(1,1)},clearcoatRoughness:{value:0},clearcoatRoughnessMap:{value:null},clearcoatRoughnessMapTransform:{value:new Ze},diffuseRoughness:{value:0},diffuseRoughnessMap:{value:null},diffuseRoughnessMapTransform:{value:new Ze},dispersion:{value:0},retroreflectivity:{value:0},iridescence:{value:0},iridescenceMap:{value:null},iridescenceMapTransform:{value:new Ze},iridescenceIOR:{value:1.3},iridescenceThicknessMinimum:{value:100},iridescenceThicknessMaximum:{value:400},iridescenceThicknessMap:{value:null},iridescenceThicknessMapTransform:{value:new Ze},sheen:{value:0},sheenColor:{value:new Ke(0)},sheenColorMap:{value:null},sheenColorMapTransform:{value:new Ze},sheenRoughness:{value:1},sheenRoughnessMap:{value:null},sheenRoughnessMapTransform:{value:new Ze},transmission:{value:0},transmissionMap:{value:null},transmissionMapTransform:{value:new Ze},transmissionSamplerSize:{value:new pe},transmissionSamplerMap:{value:null},thickness:{value:0},thicknessMap:{value:null},thicknessMapTransform:{value:new Ze},attenuationDistance:{value:0},attenuationColor:{value:new Ke(0)},specularColor:{value:new Ke(1,1,1)},specularColorMap:{value:null},specularColorMapTransform:{value:new Ze},specularIntensity:{value:1},specularIntensityMap:{value:null},specularIntensityMapTransform:{value:new Ze},anisotropyVector:{value:new pe},anisotropyMap:{value:null},anisotropyMapTransform:{value:new Ze}}]),vertexShader:nt.meshphysical_vert,fragmentShader:nt.meshphysical_frag};var al={r:0,b:0,g:0},x_=new ze,sd=new Ze;sd.set(-1,0,0,0,1,0,0,0,1);function y_(s,e,t,n,i,r){let o=new Ke(0),a=i===!0?0:1,l,c,u=null,h=0,d=null;function f(S){let v=S.isScene===!0?S.background:null;if(v&&v.isTexture){let x=S.backgroundBlurriness>0;v=e.get(v,x)}return v}function m(S){let v=!1,x=f(S);x===null?p(o,a):x&&x.isColor&&(p(x,1),v=!0);let b=s.xr.getEnvironmentBlendMode();b==="additive"?t.buffers.color.setClear(0,0,0,1,r):b==="alpha-blend"&&t.buffers.color.setClear(0,0,0,0,r),(s.autoClear||v)&&(t.buffers.depth.setTest(!0),t.buffers.depth.setMask(!0),t.buffers.color.setMask(!0),s.clear(s.autoClearColor,s.autoClearDepth,s.autoClearStencil))}function _(S,v){let x=f(v);x&&x.isCubeTexture?(c===void 0&&(c=new ke(new sn(1,1,1),new Vn({name:"BackgroundCubeMaterial",uniforms:es(Xn.backgroundCube.uniforms),vertexShader:Xn.backgroundCube.vertexShader,fragmentShader:Xn.backgroundCube.fragmentShader,side:Xt,depthTest:!1,depthWrite:!1,fog:!1,allowOverride:!1})),c.geometry.deleteAttribute("normal"),c.geometry.deleteAttribute("uv"),c.onBeforeRender=function(b,w,y){this.matrixWorld.copyPosition(y.matrixWorld)},Object.defineProperty(c.material,"envMap",{get:function(){return this.uniforms.envMap.value}}),n.update(c)),c.material.uniforms.envMap.value=x,c.material.uniforms.backgroundBlurriness.value=v.backgroundBlurriness,c.material.uniforms.backgroundIntensity.value=v.backgroundIntensity,c.material.uniforms.backgroundRotation.value.setFromMatrix4(x_.makeRotationFromEuler(v.backgroundRotation)).transpose(),x.isRenderTargetTexture===!1&&c.material.uniforms.backgroundRotation.value.premultiply(sd),c.material.toneMapped=lt.getTransfer(x.colorSpace)!==pt,(u!==x||h!==x.version||d!==s.toneMapping)&&(c.material.needsUpdate=!0,u=x,h=x.version,d=s.toneMapping),c.layers.enableAll(),S.unshift(c,c.geometry,c.material,0,0,null)):x&&x.isTexture&&(l===void 0&&(l=new ke(new Xi(2,2),new Vn({name:"BackgroundMaterial",uniforms:es(Xn.background.uniforms),vertexShader:Xn.background.vertexShader,fragmentShader:Xn.background.fragmentShader,side:Ri,depthTest:!1,depthWrite:!1,fog:!1,allowOverride:!1})),l.geometry.deleteAttribute("normal"),Object.defineProperty(l.material,"map",{get:function(){return this.uniforms.t2D.value}}),n.update(l)),l.material.uniforms.t2D.value=x,l.material.uniforms.backgroundIntensity.value=v.backgroundIntensity,l.material.toneMapped=lt.getTransfer(x.colorSpace)!==pt,x.matrixAutoUpdate===!0&&x.updateMatrix(),l.material.uniforms.uvTransform.value.copy(x.matrix),(u!==x||h!==x.version||d!==s.toneMapping)&&(l.material.needsUpdate=!0,u=x,h=x.version,d=s.toneMapping),l.layers.enableAll(),S.unshift(l,l.geometry,l.material,0,0,null))}function p(S,v){S.getRGB(al,$c(s)),t.buffers.color.setClear(al.r,al.g,al.b,v,r)}function g(){c!==void 0&&(c.geometry.dispose(),c.material.dispose(),c=void 0),l!==void 0&&(l.geometry.dispose(),l.material.dispose(),l=void 0)}return{getClearColor:function(){return o},setClearColor:function(S,v=1){o.set(S),a=v,p(o,a)},getClearAlpha:function(){return a},setClearAlpha:function(S){a=S,p(o,a)},render:m,addToRenderList:_,dispose:g}}function v_(s,e){let t=s.getParameter(s.MAX_VERTEX_ATTRIBS),n={},i=d(null),r=i,o=!1;function a(R,D,k,P,O){let G=!1,U=h(R,P,k,D);r!==U&&(r=U,c(r.object)),G=f(R,P,k,O),G&&m(R,P,k,O),O!==null&&e.update(O,s.ELEMENT_ARRAY_BUFFER),(G||o)&&(o=!1,x(R,D,k,P),O!==null&&s.bindBuffer(s.ELEMENT_ARRAY_BUFFER,e.get(O).buffer))}function l(){return s.createVertexArray()}function c(R){return s.bindVertexArray(R)}function u(R){return s.deleteVertexArray(R)}function h(R,D,k,P){let O=P.wireframe===!0,G=n[D.id];G===void 0&&(G={},n[D.id]=G);let U=R.isInstancedMesh===!0?R.id:0,ee=G[U];ee===void 0&&(ee={},G[U]=ee);let W=ee[k.id];W===void 0&&(W={},ee[k.id]=W);let K=W[O];return K===void 0&&(K=d(l()),W[O]=K),K}function d(R){let D=[],k=[],P=[];for(let O=0;O<t;O++)D[O]=0,k[O]=0,P[O]=0;return{geometry:null,program:null,wireframe:!1,newAttributes:D,enabledAttributes:k,attributeDivisors:P,object:R,attributes:{},index:null}}function f(R,D,k,P){let O=r.attributes,G=D.attributes,U=0,ee=k.getAttributes();for(let W in ee)if(ee[W].location>=0){let $=O[W],he=G[W];if(he===void 0&&(W==="instanceMatrix"&&R.instanceMatrix&&(he=R.instanceMatrix),W==="instanceColor"&&R.instanceColor&&(he=R.instanceColor)),$===void 0||$.attribute!==he||he&&$.data!==he.data)return!0;U++}return r.attributesNum!==U||r.index!==P}function m(R,D,k,P){let O={},G=D.attributes,U=0,ee=k.getAttributes();for(let W in ee)if(ee[W].location>=0){let $=G[W];$===void 0&&(W==="instanceMatrix"&&R.instanceMatrix&&($=R.instanceMatrix),W==="instanceColor"&&R.instanceColor&&($=R.instanceColor));let he={};he.attribute=$,$&&$.data&&(he.data=$.data),O[W]=he,U++}r.attributes=O,r.attributesNum=U,r.index=P}function _(){let R=r.newAttributes;for(let D=0,k=R.length;D<k;D++)R[D]=0}function p(R){g(R,0)}function g(R,D){let k=r.newAttributes,P=r.enabledAttributes,O=r.attributeDivisors;k[R]=1,P[R]===0&&(s.enableVertexAttribArray(R),P[R]=1),O[R]!==D&&(s.vertexAttribDivisor(R,D),O[R]=D)}function S(){let R=r.newAttributes,D=r.enabledAttributes;for(let k=0,P=D.length;k<P;k++)D[k]!==R[k]&&(s.disableVertexAttribArray(k),D[k]=0)}function v(R,D,k,P,O,G,U){U===!0?s.vertexAttribIPointer(R,D,k,O,G):s.vertexAttribPointer(R,D,k,P,O,G)}function x(R,D,k,P){_();let O=P.attributes,G=k.getAttributes(),U=D.defaultAttributeValues;for(let ee in G){let W=G[ee];if(W.location>=0){let K=O[ee];if(K===void 0&&(ee==="instanceMatrix"&&R.instanceMatrix&&(K=R.instanceMatrix),ee==="instanceColor"&&R.instanceColor&&(K=R.instanceColor)),K!==void 0){let $=K.normalized,he=K.itemSize,de=e.get(K);if(de===void 0)continue;let q=de.buffer,ie=de.type,ce=de.bytesPerElement,Ce=ie===s.INT||ie===s.UNSIGNED_INT||K.gpuType===wa;if(K.isInterleavedBufferAttribute){let ue=K.data,H=ue.isInterleavedBuffer===!0?ue.stride:ue.itemSize,se=K.offset;if(ue.isInstancedInterleavedBuffer||ue.isInstancedBufferAttribute){for(let fe=0;fe<W.locationSize;fe++)g(W.location+fe,ue.meshPerAttribute);R.isInstancedMesh!==!0&&P._maxInstanceCount===void 0&&(P._maxInstanceCount=ue.meshPerAttribute*ue.count)}else for(let fe=0;fe<W.locationSize;fe++)p(W.location+fe);s.bindBuffer(s.ARRAY_BUFFER,q);for(let fe=0;fe<W.locationSize;fe++)v(W.location+fe,he/W.locationSize,ie,$,H*ce,(se+he/W.locationSize*fe)*ce,Ce)}else{if(K.isInstancedBufferAttribute){for(let ue=0;ue<W.locationSize;ue++)g(W.location+ue,K.meshPerAttribute);R.isInstancedMesh!==!0&&P._maxInstanceCount===void 0&&(P._maxInstanceCount=K.meshPerAttribute*K.count)}else for(let ue=0;ue<W.locationSize;ue++)p(W.location+ue);s.bindBuffer(s.ARRAY_BUFFER,q);for(let ue=0;ue<W.locationSize;ue++)v(W.location+ue,he/W.locationSize,ie,$,he*ce,he/W.locationSize*ue*ce,Ce)}}else if(U!==void 0){let $=U[ee];if($!==void 0)switch($.length){case 2:s.vertexAttrib2fv(W.location,$);break;case 3:s.vertexAttrib3fv(W.location,$);break;case 4:s.vertexAttrib4fv(W.location,$);break;default:s.vertexAttrib1fv(W.location,$)}}}}S()}function b(){C();for(let R in n){let D=n[R];for(let k in D){let P=D[k];for(let O in P){let G=P[O];for(let U in G)u(G[U].object),delete G[U];delete P[O]}}delete n[R]}}function w(R){if(n[R.id]===void 0)return;let D=n[R.id];for(let k in D){let P=D[k];for(let O in P){let G=P[O];for(let U in G)u(G[U].object),delete G[U];delete P[O]}}delete n[R.id]}function y(R){for(let D in n){let k=n[D];for(let P in k){let O=k[P];if(O[R.id]===void 0)continue;let G=O[R.id];for(let U in G)u(G[U].object),delete G[U];delete O[R.id]}}}function M(R){for(let D in n){let k=n[D],P=R.isInstancedMesh===!0?R.id:0,O=k[P];if(O!==void 0){for(let G in O){let U=O[G];for(let ee in U)u(U[ee].object),delete U[ee];delete O[G]}delete k[P],Object.keys(k).length===0&&delete n[D]}}}function C(){E(),o=!0,r!==i&&(r=i,c(r.object))}function E(){i.geometry=null,i.program=null,i.wireframe=!1}return{setup:a,reset:C,resetDefaultState:E,dispose:b,releaseStatesOfGeometry:w,releaseStatesOfObject:M,releaseStatesOfProgram:y,initAttributes:_,enableAttribute:p,disableUnusedAttributes:S}}function S_(s,e,t){let n;function i(l){n=l}function r(l,c){s.drawArrays(n,l,c),t.update(c,n,1)}function o(l,c,u){u!==0&&(s.drawArraysInstanced(n,l,c,u),t.update(c,n,u))}function a(l,c,u){if(u===0)return;e.get("WEBGL_multi_draw").multiDrawArraysWEBGL(n,l,0,c,0,u);let d=0;for(let f=0;f<u;f++)d+=c[f];t.update(d,n,1)}this.setMode=i,this.render=r,this.renderInstances=o,this.renderMultiDraw=a}function M_(s,e,t,n){let i;function r(){if(i!==void 0)return i;if(e.has("EXT_texture_filter_anisotropic")===!0){let y=e.get("EXT_texture_filter_anisotropic");i=s.getParameter(y.MAX_TEXTURE_MAX_ANISOTROPY_EXT)}else i=0;return i}function o(y){return!(y!==Mn&&n.convert(y)!==s.getParameter(s.IMPLEMENTATION_COLOR_READ_FORMAT))}function a(y){let M=y===In&&(e.has("EXT_color_buffer_half_float")||e.has("EXT_color_buffer_float"));return!(y!==on&&y!==dn&&!M&&n.convert(y)!==s.getParameter(s.IMPLEMENTATION_COLOR_READ_TYPE))}function l(y){if(y==="highp"){if(s.getShaderPrecisionFormat(s.VERTEX_SHADER,s.HIGH_FLOAT).precision>0&&s.getShaderPrecisionFormat(s.FRAGMENT_SHADER,s.HIGH_FLOAT).precision>0)return"highp";y="mediump"}return y==="mediump"&&s.getShaderPrecisionFormat(s.VERTEX_SHADER,s.MEDIUM_FLOAT).precision>0&&s.getShaderPrecisionFormat(s.FRAGMENT_SHADER,s.MEDIUM_FLOAT).precision>0?"mediump":"lowp"}let c=t.precision!==void 0?t.precision:"highp",u=l(c);u!==c&&(Xe("WebGLRenderer:",c,"not supported, using",u,"instead."),c=u);let h=t.reversedDepthBuffer===!0&&e.has("EXT_clip_control"),d=t.logarithmicDepthBuffer===!0&&h===!1;t.reversedDepthBuffer===!0&&h===!1&&Xe("WebGLRenderer: Unable to use reversed depth buffer due to missing EXT_clip_control extension. Fallback to default depth buffer."),t.logarithmicDepthBuffer===!0&&h===!0&&Xe("WebGLRenderer: Logarithmic depth buffer is ignored when reversed depth buffer is enabled.");let f=s.getParameter(s.MAX_TEXTURE_IMAGE_UNITS),m=s.getParameter(s.MAX_VERTEX_TEXTURE_IMAGE_UNITS),_=s.getParameter(s.MAX_TEXTURE_SIZE),p=s.getParameter(s.MAX_CUBE_MAP_TEXTURE_SIZE),g=s.getParameter(s.MAX_VERTEX_ATTRIBS),S=s.getParameter(s.MAX_VERTEX_UNIFORM_VECTORS),v=s.getParameter(s.MAX_VARYING_VECTORS),x=s.getParameter(s.MAX_FRAGMENT_UNIFORM_VECTORS),b=s.getParameter(s.MAX_SAMPLES),w=s.getParameter(s.SAMPLES);return{isWebGL2:!0,getMaxAnisotropy:r,getMaxPrecision:l,textureFormatReadable:o,textureTypeReadable:a,precision:c,logarithmicDepthBuffer:d,reversedDepthBuffer:h,maxTextures:f,maxVertexTextures:m,maxTextureSize:_,maxCubemapSize:p,maxAttributes:g,maxVertexUniforms:S,maxVaryings:v,maxFragmentUniforms:x,maxSamples:b,samples:w}}function b_(s){let e=this,t=null,n=0,i=!1,r=!1,o=new tn,a=new Ze,l={value:null,needsUpdate:!1};this.uniform=l,this.numPlanes=0,this.numIntersection=0,this.init=function(h,d){let f=h.length!==0||d||n!==0||i;return i=d,n=h.length,f},this.beginShadows=function(){r=!0,u(null)},this.endShadows=function(){r=!1},this.setGlobalState=function(h,d){t=u(h,d,0)},this.setState=function(h,d,f){let m=h.clippingPlanes,_=h.clipIntersection,p=h.clipShadows,g=s.get(h);if(!i||m===null||m.length===0||r&&!p)r?u(null):c();else{let S=r?0:n,v=S*4,x=g.clippingState||null;l.value=x,x=u(m,d,v,f);for(let b=0;b!==v;++b)x[b]=t[b];g.clippingState=x,this.numIntersection=_?this.numPlanes:0,this.numPlanes+=S}};function c(){l.value!==t&&(l.value=t,l.needsUpdate=n>0),e.numPlanes=n,e.numIntersection=0}function u(h,d,f,m){let _=h!==null?h.length:0,p=null;if(_!==0){if(p=l.value,m!==!0||p===null){let g=f+_*4,S=d.matrixWorldInverse;a.getNormalMatrix(S),(p===null||p.length<g)&&(p=new Float32Array(g));for(let v=0,x=f;v!==_;++v,x+=4)o.copy(h[v]).applyMatrix4(S,a),o.normal.toArray(p,x),p[x+3]=o.constant}l.value=p,l.needsUpdate=!0}return e.numPlanes=_,e.numIntersection=0,p}}var Qr=class extends hn{constructor(e=1,t={}){super(e,e,t),this.isWebGLCubeRenderTarget=!0;let n={width:e,height:e,depth:1},i=[n,n,n,n,n,n];this.texture=new Mr(i),this._setTextureOptions(t),this.texture.isRenderTargetTexture=!0}fromEquirectangularTexture(e,t){this.texture.type=t.type,this.texture.colorSpace=t.colorSpace,this.texture.generateMipmaps=t.generateMipmaps,this.texture.minFilter=t.minFilter,this.texture.magFilter=t.magFilter;let n={uniforms:{tEquirect:{value:null}},vertexShader:`

				varying vec3 vWorldDirection;

				vec3 transformDirection( in vec3 dir, in mat4 matrix ) {

					return normalize( ( matrix * vec4( dir, 0.0 ) ).xyz );

				}

				void main() {

					vWorldDirection = transformDirection( position, modelMatrix );

					#include <begin_vertex>
					#include <project_vertex>

				}
			`,fragmentShader:`

				uniform sampler2D tEquirect;

				varying vec3 vWorldDirection;

				#include <common>

				void main() {

					vec3 direction = normalize( vWorldDirection );

					vec2 sampleUV = equirectUv( direction );

					gl_FragColor = texture2D( tEquirect, sampleUV );

				}
			`},i=new sn(5,5,5),r=new Vn({name:"CubemapFromEquirect",uniforms:es(n.uniforms),vertexShader:n.vertexShader,fragmentShader:n.fragmentShader,side:Xt,blending:$i});r.uniforms.tEquirect.value=t;let o=new ke(i,r),a=t.minFilter;return t.minFilter===fi&&(t.minFilter=Dt),new Vs(1,10,this).update(e,o),t.minFilter=a,o.geometry.dispose(),o.material.dispose(),this}clear(e,t=!0,n=!0,i=!0){let r=e.getRenderTarget();for(let o=0;o<6;o++)e.setRenderTarget(this,o),e.clear(t,n,i);e.setRenderTarget(r)}},Ff=256,w_=3,A_=20,rd=256,od=16,T_=3,E_=new L,R_=new Ke,Ys=class s{constructor(e){this._renderer=e,this._cubeSize=0,this._sourceTarget=null,this._cubeCamera=new Vs(1,10,null),this._boxMesh=new ke(new sn(5,5,5),null),this._cubemapMaterial=null,this._equirectMaterial=null,this._blurMaterial=null,this._ggxMaterial=null,this._integrationMaterial=null}fromScene(e,t=0,n=.1,i=100,r={}){let{size:o=256,position:a=E_}=r,l=this._renderer;this._setSize(o);let c=this._allocateTarget(),u=this._getSourceTarget(!0);t>0&&(u.texture.mipmapsAutoUpdate=!1);let h=l.autoClear,d=l.autoClearColor,f=l.autoClearDepth,m=l.autoClearStencil,_=e.background;l.autoClear=!0,l.autoClearColor=!0,l.autoClearDepth=!0,l.autoClearStencil=!0,_===null&&(e.background=l.getClearColor(R_));let p=new Vs(n,i,u);return p.position.copy(a),p.update(l,e),l.autoClear=h,l.autoClearColor=d,l.autoClearDepth=f,l.autoClearStencil=m,e.background=_,t>0&&(u.texture.mipmapsAutoUpdate=!0,this._blur(c,t)),this._applyPMREM(c),c}fromEquirectangular(e,t=null){return this._fromTexture(e,t)}fromCubemap(e,t=null){return this._fromTexture(e,t)}compileCubemapShader(){this._cubemapMaterial===null&&(this._cubemapMaterial=zf(),this._compileMaterial(this._cubemapMaterial))}compileEquirectangularShader(){this._equirectMaterial===null&&(this._equirectMaterial=Bf(),this._compileMaterial(this._equirectMaterial))}dispose(){this._sourceTarget!==null&&this._sourceTarget.dispose(),this._cubemapMaterial!==null&&this._cubemapMaterial.dispose(),this._equirectMaterial!==null&&this._equirectMaterial.dispose(),this._blurMaterial!==null&&this._blurMaterial.dispose(),this._ggxMaterial!==null&&this._ggxMaterial.dispose(),this._integrationMaterial!==null&&this._integrationMaterial.dispose(),this._boxMesh.geometry.dispose()}static lodToRoughness(e,t){return t>0?1-Math.sqrt(1-e/t):0}_setSize(e){this._cubeSize=Math.max(Ff,Ef(e))}_fromTexture(e,t){e.mapping===ji||e.mapping===Ws?this._setSize(e.image.length===0?Ff:e.image[0].width||e.image[0].image.width):this._setSize(e.image.width/4);let n=t||this._allocateTarget();return this._textureToCubemap(e),this._applyPMREM(n),n}_allocateTarget(){let e=this._cubeSize,t=Of(e,!1,!1),n=Math.log2(e)-w_;for(let i=0;i<=n;i++)t.texture.mipmaps.push({width:e>>i,height:e>>i});return t.texture.name="PMREM",t.texture.isPMREMTexture=!0,t}_getSourceTarget(e=!1){let t=this._cubeSize,n=this._sourceTarget;return(n===null||n.width!==t||e&&n.depthBuffer===!1)&&(n!==null&&n.dispose(),this._sourceTarget=Of(t,!0,e)),this._sourceTarget}_compileMaterial(e){this._boxMesh.material=e,this._renderer.compile(this._boxMesh,this._cubeCamera.children[0])}_renderCube(e,t,n){let i=this._boxMesh,r=this._cubeCamera;i.material=n,r.renderTarget=e,r.activeMipmapLevel=t;let o=e.width>>t;e.viewport.set(0,0,o,o),r.update(this._renderer,i),e.viewport.set(0,0,e.width,e.height)}_textureToCubemap(e){let t;e.mapping===ji||e.mapping===Ws?(this._cubemapMaterial===null&&(this._cubemapMaterial=zf()),t=this._cubemapMaterial,t.uniforms.flipEnvMap.value=e.isRenderTargetTexture===!1?-1:1):(this._equirectMaterial===null&&(this._equirectMaterial=Bf()),t=this._equirectMaterial),t.uniforms.envMap.value=e,this._renderCube(this._getSourceTarget(),0,t)}_applyPMREM(e){this._ggxMaterial===null&&(this._ggxMaterial=C_(),this._integrationMaterial=P_());let t=this._cubeSize,n=e.texture.mipmaps.length-1,i=this._ggxMaterial.uniforms;i.envMap.value=this._sourceTarget.texture;let r=this._integrationMaterial.uniforms;r.envMap.value=this._sourceTarget.texture,r.sourceLod.value=Math.log2(t/od);for(let o=0;o<=n;o++){let a=s.lodToRoughness(o,n);if(o>n-T_)r.roughness.value=a,this._renderCube(e,o,this._integrationMaterial);else{let l=a>0?Math.log2(t)+.5*Math.log2(6/(rd*Math.pow(a,4)))+.5:0;i.roughness.value=a,i.lodBias.value=l,this._renderCube(e,o,this._ggxMaterial)}}}_blur(e,t){this._blurMaterial===null&&(this._blurMaterial=I_());let n=this._blurMaterial,i=n.uniforms,r=this._sourceTarget;i.sigma.value=Math.min(t,Math.PI)/Math.SQRT2,i.envMap.value=r.texture,this._renderCube(e,0,n),i.envMap.value=e.texture,this._renderCube(r,0,n)}};function Of(s,e,t){return new Qr(s,{minFilter:fi,generateMipmaps:e,type:In,colorSpace:Gt,depthBuffer:t})}function eo(s,e,t){return new Vn({name:s,uniforms:e,vertexShader:nt.cube_vert,fragmentShader:t,side:Xt,blending:$i,depthTest:!1,depthWrite:!1})}function C_(){return eo("PMREMGGXConvolution",{envMap:{value:null},roughness:{value:0},lodBias:{value:0}},`

		#define GGX_SAMPLES ${rd}u

		varying vec3 vWorldDirection;

		uniform samplerCube envMap;
		uniform float roughness;
		uniform float lodBias;

		#include <common>

		// Van der Corput radical inverse
		float radicalInverse_VdC( uint bits ) {

			bits = ( bits << 16u ) | ( bits >> 16u );
			bits = ( ( bits & 0x55555555u ) << 1u ) | ( ( bits & 0xAAAAAAAAu ) >> 1u );
			bits = ( ( bits & 0x33333333u ) << 2u ) | ( ( bits & 0xCCCCCCCCu ) >> 2u );
			bits = ( ( bits & 0x0F0F0F0Fu ) << 4u ) | ( ( bits & 0xF0F0F0F0u ) >> 4u );
			bits = ( ( bits & 0x00FF00FFu ) << 8u ) | ( ( bits & 0xFF00FF00u ) >> 8u );
			return float( bits ) * 2.3283064365386963e-10; // / 0x100000000

		}

		// Hammersley sequence
		vec2 hammersley( uint i, uint N ) {

			return vec2( float( i ) / float( N ), radicalInverse_VdC( i ) );

		}

		void main() {

			vec3 N = normalize( vWorldDirection );

			// For very low roughness, just sample the environment directly
			if ( roughness < 0.001 ) {

				gl_FragColor = vec4( textureLod( envMap, N, 0.0 ).rgb, 1.0 );
				return;

			}

			float alpha = roughness * roughness;
			float alpha2 = alpha * alpha;

			// Tangent space basis for VNDF sampling
			vec3 up = abs( N.z ) < 0.999 ? vec3( 0.0, 0.0, 1.0 ) : vec3( 1.0, 0.0, 0.0 );
			vec3 tangent = normalize( cross( up, N ) );
			vec3 bitangent = cross( N, tangent );

			vec3 prefilteredColor = vec3( 0.0 );
			float totalWeight = 0.0;

			for ( uint i = 0u; i < GGX_SAMPLES; i ++ ) {

				vec2 Xi = hammersley( i, GGX_SAMPLES );

				// With V = N, sample the reflected direction directly.
				float invQ = 1.0 / ( 1.0 - Xi.x + alpha2 * Xi.x );
				float NdotL = ( 1.0 - Xi.x - alpha2 * Xi.x ) * invQ;

				if ( NdotL > 0.0 ) {

					float phi = 2.0 * PI * Xi.y;
					float sinTheta = 2.0 * alpha * sqrt( Xi.x * ( 1.0 - Xi.x ) ) * invQ;
					vec3 L = N * NdotL + ( tangent * cos( phi ) + bitangent * sin( phi ) ) * sinTheta;

					// Match the source mip to the sample's solid angle. The terms
					// independent of the sample are precomputed in lodBias.
					float d = alpha2 * invQ;
					float lod = max( log2( d ) + lodBias, 0.0 );

					// Weight by NdotL for the split-sum approximation
					prefilteredColor += textureLod( envMap, L, lod ).rgb * NdotL;
					totalWeight += NdotL;

				}

			}

			gl_FragColor = vec4( prefilteredColor / totalWeight, 1.0 );

		}
	`)}function P_(){return eo("PMREMGGXIntegration",{envMap:{value:null},roughness:{value:0},sourceLod:{value:0},sourceSize:{value:od}},`

		varying vec3 vWorldDirection;

		uniform samplerCube envMap;
		uniform float roughness;
		uniform float sourceLod;
		uniform int sourceSize;

		void main() {

			vec3 N = normalize( vWorldDirection );

			float alpha = roughness * roughness;
			float alpha2 = alpha * alpha;

			float texelSize = 2.0 / float( sourceSize );

			vec3 prefilteredColor = vec3( 0.0 );
			float totalWeight = 0.0;

			// Pair opposite texels: only the one in N's hemisphere contributes.
			for ( int face = 0; face < 3; face ++ ) {

				for ( int y = 0; y < sourceSize; y ++ ) {

					for ( int x = 0; x < sourceSize; x ++ ) {

						vec2 uv = ( vec2( x, y ) + 0.5 ) * texelSize - 1.0;
						vec3 texelDirection = face == 0 ? vec3( 1.0, uv ) : ( face == 1 ? vec3( uv.x, 1.0, uv.y ) : vec3( uv, 1.0 ) );

						float invDistance = inversesqrt( 1.0 + dot( uv, uv ) );
						float NdotL = dot( N, texelDirection );
						texelDirection *= NdotL < 0.0 ? - 1.0 : 1.0;
						NdotL = abs( NdotL ) * invDistance;

						// With V = N, NdotH squared is ( 1 + NdotL ) / 2. Common factors
						// in the GGX distribution and texel solid angle cancel when normalized.
						float d = 1.0 + alpha2 + ( alpha2 - 1.0 ) * NdotL;
						float weight = NdotL * invDistance * invDistance * invDistance / ( d * d );

						prefilteredColor += textureLod( envMap, texelDirection, sourceLod ).rgb * weight;
						totalWeight += weight;

					}

				}

			}

			gl_FragColor = vec4( prefilteredColor / totalWeight, 1.0 );

		}
	`)}function I_(){return eo("PMREMSphericalGaussianBlur",{envMap:{value:null},sigma:{value:0}},`

		#define SAMPLES ${A_}
		#define GOLDEN_ANGLE 2.39996322973

		varying vec3 vWorldDirection;

		uniform samplerCube envMap;
		uniform float sigma;

		#include <common>

		void main() {

			vec3 outputDirection = normalize( vWorldDirection );

			vec3 up = abs( outputDirection.z ) < 0.999 ? vec3( 0.0, 0.0, 1.0 ) : vec3( 1.0, 0.0, 0.0 );
			vec3 tangent = normalize( cross( up, outputDirection ) );
			vec3 bitangent = cross( outputDirection, tangent );

			// Truncate the kernel at three standard deviations or at the antipode.
			float thetaMax = min( 3.0 * sigma, PI );
			float truncation = 1.0 - exp( - 0.5 * thetaMax * thetaMax / ( sigma * sigma ) );

			vec3 accumColor = vec3( 0.0 );
			float accumWeight = 0.0;

			for ( int i = 0; i < SAMPLES; i ++ ) {

				// Stratified inverse-CDF sampling of the Gaussian, placed on a golden-angle spiral.
				float stratum = ( float( i ) + 0.5 ) / float( SAMPLES );
				float theta = sigma * sqrt( - 2.0 * log( 1.0 - stratum * truncation ) );
				float phi = float( i ) * GOLDEN_ANGLE;

				vec3 offset = cos( phi ) * tangent + sin( phi ) * bitangent;
				vec3 sampleDirection = cos( theta ) * outputDirection + sin( theta ) * offset;

				// Correct the planar sample density to solid angle.
				float weight = sin( theta ) / theta;

				accumColor += weight * textureLod( envMap, sampleDirection, 0.0 ).rgb;
				accumWeight += weight;

			}

			gl_FragColor = vec4( accumColor / accumWeight, 1.0 );

		}
	`)}function Bf(){return eo("PMREMEquirectangularToCubemap",{envMap:{value:null}},`

		varying vec3 vWorldDirection;

		uniform sampler2D envMap;

		#include <common>

		void main() {

			// Average four subpixel samples to preserve small, bright features.
			vec3 direction = normalize( vWorldDirection );
			vec3 dx = dFdx( direction ) * 0.25;
			vec3 dy = dFdy( direction ) * 0.25;

			vec3 color = textureLod( envMap, equirectUv( normalize( direction - dx - dy ) ), 0.0 ).rgb;
			color += textureLod( envMap, equirectUv( normalize( direction + dx - dy ) ), 0.0 ).rgb;
			color += textureLod( envMap, equirectUv( normalize( direction - dx + dy ) ), 0.0 ).rgb;
			color += textureLod( envMap, equirectUv( normalize( direction + dx + dy ) ), 0.0 ).rgb;
			gl_FragColor = vec4( color * 0.25, 1.0 );

		}
	`)}function zf(){return eo("PMREMCubemapToCubemap",{envMap:{value:null},flipEnvMap:{value:-1}},`

		varying vec3 vWorldDirection;

		uniform samplerCube envMap;
		uniform float flipEnvMap;

		void main() {

			gl_FragColor = vec4( textureCube( envMap, vec3( flipEnvMap * vWorldDirection.x, vWorldDirection.yz ) ).rgb, 1.0 );

		}
	`)}function D_(s){let e=new WeakMap,t=new WeakMap,n=null;function i(d,f=!1){return d==null?null:f?o(d):r(d)}function r(d){if(d&&d.isTexture){let f=d.mapping;if(f===Sa||f===Ma)if(e.has(d)){let m=e.get(d).texture;return a(m,d.mapping)}else{let m=d.image;if(m&&m.height>0){let _=new Qr(m.height);return _.fromEquirectangularTexture(s,d),e.set(d,_),d.addEventListener("dispose",c),a(_.texture,d.mapping)}else return null}}return d}function o(d){if(d&&d.isTexture&&d.isPMREMTexture!==!0){let f=d.mapping,m=f===Sa||f===Ma,_=f===ji||f===Ws;if(m||_){let p=t.get(d),g=p!==void 0?p.texture.pmremVersion:0;if(d.isRenderTargetTexture&&d.pmremVersion!==g)return n===null&&(n=new Ys(s)),p=m?n.fromEquirectangular(d,p):n.fromCubemap(d,p),p.texture.pmremVersion=d.pmremVersion,t.set(d,p),p.texture;if(p!==void 0)return p.texture;{let S=d.image;return m&&S&&S.height>0||_&&S&&l(S)?(n===null&&(n=new Ys(s)),p=m?n.fromEquirectangular(d):n.fromCubemap(d),p.texture.pmremVersion=d.pmremVersion,t.set(d,p),d.addEventListener("dispose",u),p.texture):null}}}return d}function a(d,f){return f===Sa?d.mapping=ji:f===Ma&&(d.mapping=Ws),d}function l(d){let f=0,m=6;for(let _=0;_<m;_++)d[_]!==void 0&&f++;return f===m}function c(d){let f=d.target;f.removeEventListener("dispose",c);let m=e.get(f);m!==void 0&&(e.delete(f),m.dispose())}function u(d){let f=d.target;f.removeEventListener("dispose",u);let m=t.get(f);m!==void 0&&(t.delete(f),m.dispose())}function h(){e=new WeakMap,t=new WeakMap,n!==null&&(n.dispose(),n=null)}return{get:i,dispose:h}}function L_(s){let e={};function t(n){if(e[n]!==void 0)return e[n];let i=s.getExtension(n);return e[n]=i,i}return{has:function(n){return t(n)!==null},init:function(){t("EXT_color_buffer_float"),t("WEBGL_clip_cull_distance"),t("OES_texture_float_linear"),t("EXT_color_buffer_half_float"),t("WEBGL_multisampled_render_to_texture"),t("WEBGL_render_shared_exponent")},get:function(n){let i=t(n);return i===null&&Vi("WebGLRenderer: "+n+" extension not supported."),i}}}function N_(s,e,t,n){let i={},r=new WeakMap,o=new FinalizationRegistry(m=>delete i[m]);function a(m){l(m.target)}function l(m){m.index!==null&&e.remove(m.index);for(let p in m.attributes)e.remove(m.attributes[p]);m.removeEventListener("dispose",a),delete i[m.id],o.unregister(m);let _=r.get(m);_&&(e.remove(_),r.delete(m)),n.releaseStatesOfGeometry(m),m.isInstancedBufferGeometry===!0&&delete m._maxInstanceCount,t.memory.geometries--}function c(m,_){return i[_.id]!==void 0||(_.addEventListener("dispose",a),i[_.id]=new WeakRef(_),o.register(_,_.id,_),t.memory.geometries++),_}function u(m){let _=m.attributes;for(let p in _)e.update(_[p],s.ARRAY_BUFFER)}function h(m){let _=m.index,p=m.attributes.position,g=0;if(p===void 0)return;let S=_!==null?_.array.length:p.array.length/3-1,v=p.count>=65535?xr:_r,x=new v(Math.ceil(S/3)*6,1),b=x.array;if(_!==null){let y=_.array;g=_.version;for(let M=0,C=0;M<S;M+=3){let E=y[M+0],R=y[M+1],D=y[M+2];b[C++]=E,b[C++]=R,b[C++]=R,b[C++]=D,b[C++]=D,b[C++]=E}}else{g=p.version;for(let y=0,M=0;y<S;y+=3){let C=y+0,E=y+1,R=y+2;b[M++]=C,b[M++]=E,b[M++]=E,b[M++]=R,b[M++]=R,b[M++]=C}}x.version=g;let w=r.get(m);w&&e.remove(w),r.set(m,x)}function d(m){let _=r.get(m);if(_){let p=m.index;p!==null&&_.version<p.version&&h(m)}else h(m);return r.get(m)}function f(){for(let m in i){let _=i[m].deref();_!==void 0&&l(_)}}return{get:c,update:u,getWireframeAttribute:d,dispose:f}}function U_(s,e,t){let n;function i(h){n=h}let r,o;function a(h){r=h.type,o=h.bytesPerElement}function l(h,d){s.drawElements(n,d,r,h*o),t.update(d,n,1)}function c(h,d,f){f!==0&&(s.drawElementsInstanced(n,d,r,h*o,f),t.update(d,n,f))}function u(h,d,f){if(f===0)return;e.get("WEBGL_multi_draw").multiDrawElementsWEBGL(n,d,0,r,h,0,f);let _=0;for(let p=0;p<f;p++)_+=d[p];t.update(_,n,1)}this.setMode=i,this.setIndex=a,this.render=l,this.renderInstances=c,this.renderMultiDraw=u}function F_(s){let e={geometries:0,textures:0},t={frame:0,calls:0,triangles:0,points:0,lines:0};function n(r,o,a){switch(t.calls++,o){case s.TRIANGLES:t.triangles+=a*(r/3);break;case s.LINES:t.lines+=a*(r/2);break;case s.LINE_STRIP:t.lines+=a*(r-1);break;case s.LINE_LOOP:t.lines+=a*r;break;case s.POINTS:t.points+=a*r;break;default:Ye("WebGLInfo: Unknown draw mode:",o);break}}function i(){t.calls=0,t.triangles=0,t.points=0,t.lines=0}return{memory:e,render:t,programs:null,autoReset:!0,reset:i,update:n}}function O_(s,e,t){let n=new WeakMap,i=new _t;function r(o,a,l){let c=o.morphTargetInfluences,u=a.morphAttributes.position||a.morphAttributes.normal||a.morphAttributes.color,h=u!==void 0?u.length:0,d=n.get(a);if(d===void 0||d.count!==h){let C=function(){y.dispose(),n.delete(a),a.removeEventListener("dispose",C)};d!==void 0&&d.texture.dispose();let f=a.morphAttributes.position!==void 0,m=a.morphAttributes.normal!==void 0,_=a.morphAttributes.color!==void 0,p=a.morphAttributes.position||[],g=a.morphAttributes.normal||[],S=a.morphAttributes.color||[],v=0;f===!0&&(v=1),m===!0&&(v=2),_===!0&&(v=3);let x=a.attributes.position.count*v,b=1;x>e.maxTextureSize&&(b=Math.ceil(x/e.maxTextureSize),x=e.maxTextureSize);let w=new Float32Array(x*b*4*h),y=new gr(w,x,b,h);y.type=dn,y.needsUpdate=!0;let M=v*4;for(let E=0;E<h;E++){let R=p[E],D=g[E],k=S[E],P=x*b*4*E;for(let O=0;O<R.count;O++){let G=O*M;f===!0&&(i.fromBufferAttribute(R,O),w[P+G+0]=i.x,w[P+G+1]=i.y,w[P+G+2]=i.z,w[P+G+3]=0),m===!0&&(i.fromBufferAttribute(D,O),w[P+G+4]=i.x,w[P+G+5]=i.y,w[P+G+6]=i.z,w[P+G+7]=0),_===!0&&(i.fromBufferAttribute(k,O),w[P+G+8]=i.x,w[P+G+9]=i.y,w[P+G+10]=i.z,w[P+G+11]=k.itemSize===4?i.w:1)}}d={count:h,texture:y,size:new pe(x,b)},n.set(a,d),a.addEventListener("dispose",C)}if(o.isInstancedMesh===!0&&o.morphTexture!==null)l.getUniforms().setValue(s,"morphTexture",o.morphTexture,t);else{let f=0;for(let _=0;_<c.length;_++)f+=c[_];let m=a.morphTargetsRelative?1:1-f;l.getUniforms().setValue(s,"morphTargetBaseInfluence",m),l.getUniforms().setValue(s,"morphTargetInfluences",c)}l.getUniforms().setValue(s,"morphTargetsTexture",d.texture,t),l.getUniforms().setValue(s,"morphTargetsTextureSize",d.size)}return{update:r}}function B_(s,e,t,n,i){let r=new WeakMap;function o(c){let u=i.render.frame,h=c.geometry,d=e.get(c,h);if(r.get(d)!==u&&(e.update(d),r.set(d,u)),c.isInstancedMesh&&(c.hasEventListener("dispose",l)===!1&&c.addEventListener("dispose",l),r.get(c)!==u&&(t.update(c.instanceMatrix,s.ARRAY_BUFFER),c.instanceColor!==null&&t.update(c.instanceColor,s.ARRAY_BUFFER),r.set(c,u))),c.isSkinnedMesh){let f=c.skeleton;r.get(f)!==u&&(f.update(),r.set(f,u))}return d}function a(){r=new WeakMap}function l(c){let u=c.target;u.removeEventListener("dispose",l),n.releaseStatesOfObject(u),t.remove(u.instanceMatrix),u.instanceColor!==null&&t.remove(u.instanceColor)}return{update:o,dispose:a}}var z_={[Cc]:"LINEAR_TONE_MAPPING",[Pc]:"REINHARD_TONE_MAPPING",[Ic]:"CINEON_TONE_MAPPING",[Dc]:"ACES_FILMIC_TONE_MAPPING",[Nc]:"AGX_TONE_MAPPING",[Uc]:"NEUTRAL_TONE_MAPPING",[Lc]:"CUSTOM_TONE_MAPPING"};function k_(s,e,t,n,i,r){let o=new hn(e,t,{type:s,depthBuffer:i,stencilBuffer:r,samples:n?4:0,storeMultisampledDepthBuffer:!1,storeMultisampledStencilBuffer:!1,resolveDepthBuffer:!1,resolveStencilBuffer:!1}),a=null,l=null,c=null,u=new Lt;u.setAttribute("position",new ct([-1,3,0,-1,-1,0,3,-1,0],3)),u.setAttribute("uv",new ct([0,2,0,0,2,0],2));let h=new ca({uniforms:{tDiffuse:{value:null}},vertexShader:`
			precision highp float;

			uniform mat4 modelViewMatrix;
			uniform mat4 projectionMatrix;

			attribute vec3 position;
			attribute vec2 uv;

			varying vec2 vUv;

			void main() {
				vUv = uv;
				gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );
			}`,fragmentShader:`
			precision highp float;

			uniform sampler2D tDiffuse;

			varying vec2 vUv;

			#include <tonemapping_pars_fragment>
			#include <colorspace_pars_fragment>

			void main() {
				gl_FragColor = texture2D( tDiffuse, vUv );

				#ifdef LINEAR_TONE_MAPPING
					gl_FragColor.rgb = LinearToneMapping( gl_FragColor.rgb );
				#elif defined( REINHARD_TONE_MAPPING )
					gl_FragColor.rgb = ReinhardToneMapping( gl_FragColor.rgb );
				#elif defined( CINEON_TONE_MAPPING )
					gl_FragColor.rgb = CineonToneMapping( gl_FragColor.rgb );
				#elif defined( ACES_FILMIC_TONE_MAPPING )
					gl_FragColor.rgb = ACESFilmicToneMapping( gl_FragColor.rgb );
				#elif defined( AGX_TONE_MAPPING )
					gl_FragColor.rgb = AgXToneMapping( gl_FragColor.rgb );
				#elif defined( NEUTRAL_TONE_MAPPING )
					gl_FragColor.rgb = NeutralToneMapping( gl_FragColor.rgb );
				#elif defined( CUSTOM_TONE_MAPPING )
					gl_FragColor.rgb = CustomToneMapping( gl_FragColor.rgb );
				#endif

				#ifdef SRGB_TRANSFER
					gl_FragColor = sRGBTransferOETF( gl_FragColor );
				#endif
			}`,depthTest:!1,depthWrite:!1}),d=new ke(u,h),f=new Yi(-1,1,1,-1,0,1),m=null,_=null,p=!1,g,S=null,v=[],x=!1;this.setSize=function(b,w){o.setSize(b,w),a!==null&&a.setSize(b,w),l!==null&&l.setSize(b,w);for(let y=0;y<v.length;y++){let M=v[y];M.setSize&&M.setSize(b,w)}},this.setEffects=function(b){v=b,x=v.length>0&&v[0].isRenderPass===!0;let w=o.width,y=o.height;v.length>0&&a===null&&(a=new hn(w,y,{type:In,depthBuffer:!1,stencilBuffer:!1}),l=new hn(w,y,{type:In,depthBuffer:!1,stencilBuffer:!1}));let M=i===!0&&v.some(C=>C.needsDepthTexture===!0);M!==(o.depthTexture!==null)&&(M===!0&&c===null&&(c=new oi(w,y),r===!0&&(c.format=di,c.type=Qi)),o.dispose(),o.depthTexture=M?c:null,o.resolveDepthBuffer=M,o.storeMultisampledDepthBuffer=M);for(let C=0;C<v.length;C++){let E=v[C];E.setSize&&E.setSize(w,y),M&&E.needsDepthTexture===!0&&E.setDepthTexture&&E.setDepthTexture(c)}},this.begin=function(b,w){if(p||b.toneMapping===Hn&&v.length===0)return!1;if(S=w,w!==null){let y=w.width,M=w.height;(o.width!==y||o.height!==M)&&this.setSize(y,M)}return x===!1&&b.setRenderTarget(o),g=b.toneMapping,b.toneMapping=Hn,!0},this.hasRenderPass=function(){return x},this.end=function(b,w){b.toneMapping=g,p=!0;let y=o,M=a;for(let C=0;C<v.length;C++){let E=v[C];E.enabled!==!1&&(E.render(b,M,y,w),E.needsSwap!==!1&&(y=M,M=M===a?l:a))}if(m!==b.outputColorSpace||_!==b.toneMapping){m=b.outputColorSpace,_=b.toneMapping,h.defines={},lt.getTransfer(m)===pt&&(h.defines.SRGB_TRANSFER="");let C=z_[_];C&&(h.defines[C]=""),h.needsUpdate=!0}h.uniforms.tDiffuse.value=y.texture,b.setRenderTarget(S),b.render(d,f),S=null,p=!1},this.isCompositing=function(){return p},this.dispose=function(){o.dispose(),a!==null&&a.dispose(),l!==null&&l.dispose(),u.dispose(),h.dispose()}}var ad=new Nt,ld=new oi(1,1),cd=new br(1),hd=new gr,ud=new ta,fd=new Mr,kf=[],Vf=[],Gf=new Float32Array(16),Hf=new Float32Array(9),Wf=new Float32Array(4);function Js(s,e,t){let n=s[0];if(n<=0||n>0)return s;let i=e*t,r=kf[i];if(r===void 0&&(r=new Float32Array(i),kf[i]=r),e!==0){n.toArray(r,0);for(let o=1,a=0;o!==e;++o)a+=t,s[o].toArray(r,a)}return r}function Ft(s,e){if(s.length!==e.length)return!1;for(let t=0,n=s.length;t<n;t++)if(s[t]!==e[t])return!1;return!0}function Ot(s,e){for(let t=0,n=e.length;t<n;t++)s[t]=e[t]}function cl(s,e){let t=Vf[e];t===void 0&&(t=new Int32Array(e),Vf[e]=t);for(let n=0;n!==e;++n)t[n]=s.allocateTextureUnit();return t}function V_(s,e){let t=this.cache;t[0]!==e&&(s.uniform1f(this.addr,e),t[0]=e)}function G_(s,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y)&&(s.uniform2f(this.addr,e.x,e.y),t[0]=e.x,t[1]=e.y);else{if(Ft(t,e))return;s.uniform2fv(this.addr,e),Ot(t,e)}}function H_(s,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y||t[2]!==e.z)&&(s.uniform3f(this.addr,e.x,e.y,e.z),t[0]=e.x,t[1]=e.y,t[2]=e.z);else if(e.r!==void 0)(t[0]!==e.r||t[1]!==e.g||t[2]!==e.b)&&(s.uniform3f(this.addr,e.r,e.g,e.b),t[0]=e.r,t[1]=e.g,t[2]=e.b);else{if(Ft(t,e))return;s.uniform3fv(this.addr,e),Ot(t,e)}}function W_(s,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y||t[2]!==e.z||t[3]!==e.w)&&(s.uniform4f(this.addr,e.x,e.y,e.z,e.w),t[0]=e.x,t[1]=e.y,t[2]=e.z,t[3]=e.w);else{if(Ft(t,e))return;s.uniform4fv(this.addr,e),Ot(t,e)}}function X_(s,e){let t=this.cache,n=e.elements;if(n===void 0){if(Ft(t,e))return;s.uniformMatrix2fv(this.addr,!1,e),Ot(t,e)}else{if(Ft(t,n))return;Wf.set(n),s.uniformMatrix2fv(this.addr,!1,Wf),Ot(t,n)}}function q_(s,e){let t=this.cache,n=e.elements;if(n===void 0){if(Ft(t,e))return;s.uniformMatrix3fv(this.addr,!1,e),Ot(t,e)}else{if(Ft(t,n))return;Hf.set(n),s.uniformMatrix3fv(this.addr,!1,Hf),Ot(t,n)}}function Y_(s,e){let t=this.cache,n=e.elements;if(n===void 0){if(Ft(t,e))return;s.uniformMatrix4fv(this.addr,!1,e),Ot(t,e)}else{if(Ft(t,n))return;Gf.set(n),s.uniformMatrix4fv(this.addr,!1,Gf),Ot(t,n)}}function Z_(s,e){let t=this.cache;t[0]!==e&&(s.uniform1i(this.addr,e),t[0]=e)}function J_(s,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y)&&(s.uniform2i(this.addr,e.x,e.y),t[0]=e.x,t[1]=e.y);else{if(Ft(t,e))return;s.uniform2iv(this.addr,e),Ot(t,e)}}function $_(s,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y||t[2]!==e.z)&&(s.uniform3i(this.addr,e.x,e.y,e.z),t[0]=e.x,t[1]=e.y,t[2]=e.z);else{if(Ft(t,e))return;s.uniform3iv(this.addr,e),Ot(t,e)}}function K_(s,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y||t[2]!==e.z||t[3]!==e.w)&&(s.uniform4i(this.addr,e.x,e.y,e.z,e.w),t[0]=e.x,t[1]=e.y,t[2]=e.z,t[3]=e.w);else{if(Ft(t,e))return;s.uniform4iv(this.addr,e),Ot(t,e)}}function j_(s,e){let t=this.cache;t[0]!==e&&(s.uniform1ui(this.addr,e),t[0]=e)}function Q_(s,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y)&&(s.uniform2ui(this.addr,e.x,e.y),t[0]=e.x,t[1]=e.y);else{if(Ft(t,e))return;s.uniform2uiv(this.addr,e),Ot(t,e)}}function ex(s,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y||t[2]!==e.z)&&(s.uniform3ui(this.addr,e.x,e.y,e.z),t[0]=e.x,t[1]=e.y,t[2]=e.z);else{if(Ft(t,e))return;s.uniform3uiv(this.addr,e),Ot(t,e)}}function tx(s,e){let t=this.cache;if(e.x!==void 0)(t[0]!==e.x||t[1]!==e.y||t[2]!==e.z||t[3]!==e.w)&&(s.uniform4ui(this.addr,e.x,e.y,e.z,e.w),t[0]=e.x,t[1]=e.y,t[2]=e.z,t[3]=e.w);else{if(Ft(t,e))return;s.uniform4uiv(this.addr,e),Ot(t,e)}}function hl(s){return s.version===0&&(s.compareFunction=Xc,s.needsUpdate=!0),s}function nx(s,e,t){let n=this.cache,i=t.allocateTextureUnit();n[0]!==i&&(s.uniform1i(this.addr,i),n[0]=i);let r;this.type===s.SAMPLER_2D_SHADOW?r=hl(ld):r=ad,t.setTexture2D(e||r,i)}function ix(s,e,t){let n=this.cache,i=t.allocateTextureUnit();n[0]!==i&&(s.uniform1i(this.addr,i),n[0]=i),t.setTexture3D(e||ud,i)}function sx(s,e,t){let n=this.cache,i=t.allocateTextureUnit();n[0]!==i&&(s.uniform1i(this.addr,i),n[0]=i);let r;this.type===s.SAMPLER_CUBE_SHADOW?r=hl(cd):r=fd,t.setTextureCube(e||r,i)}function rx(s,e,t){let n=this.cache,i=t.allocateTextureUnit();n[0]!==i&&(s.uniform1i(this.addr,i),n[0]=i),t.setTexture2DArray(e||hd,i)}function ox(s){switch(s){case 5126:return V_;case 35664:return G_;case 35665:return H_;case 35666:return W_;case 35674:return X_;case 35675:return q_;case 35676:return Y_;case 5124:case 35670:return Z_;case 35667:case 35671:return J_;case 35668:case 35672:return $_;case 35669:case 35673:return K_;case 5125:return j_;case 36294:return Q_;case 36295:return ex;case 36296:return tx;case 35678:case 36198:case 36298:case 36306:case 35682:return nx;case 35679:case 36299:case 36307:return ix;case 35680:case 36300:case 36308:case 36293:return sx;case 36289:case 36303:case 36311:case 36292:return rx}}function ax(s,e){s.uniform1fv(this.addr,e)}function lx(s,e){let t=Js(e,this.size,2);s.uniform2fv(this.addr,t)}function cx(s,e){let t=Js(e,this.size,3);s.uniform3fv(this.addr,t)}function hx(s,e){let t=Js(e,this.size,4);s.uniform4fv(this.addr,t)}function ux(s,e){let t=Js(e,this.size,4);s.uniformMatrix2fv(this.addr,!1,t)}function fx(s,e){let t=Js(e,this.size,9);s.uniformMatrix3fv(this.addr,!1,t)}function dx(s,e){let t=Js(e,this.size,16);s.uniformMatrix4fv(this.addr,!1,t)}function px(s,e){s.uniform1iv(this.addr,e)}function mx(s,e){s.uniform2iv(this.addr,e)}function gx(s,e){s.uniform3iv(this.addr,e)}function _x(s,e){s.uniform4iv(this.addr,e)}function xx(s,e){s.uniform1uiv(this.addr,e)}function yx(s,e){s.uniform2uiv(this.addr,e)}function vx(s,e){s.uniform3uiv(this.addr,e)}function Sx(s,e){s.uniform4uiv(this.addr,e)}function Mx(s,e,t){let n=this.cache,i=e.length,r=cl(t,i);Ft(n,r)||(s.uniform1iv(this.addr,r),Ot(n,r));let o;this.type===s.SAMPLER_2D_SHADOW?o=hl(ld):o=ad;for(let a=0;a!==i;++a)t.setTexture2D(e[a]||o,r[a])}function bx(s,e,t){let n=this.cache,i=e.length,r=cl(t,i);Ft(n,r)||(s.uniform1iv(this.addr,r),Ot(n,r));for(let o=0;o!==i;++o)t.setTexture3D(e[o]||ud,r[o])}function wx(s,e,t){let n=this.cache,i=e.length,r=cl(t,i);Ft(n,r)||(s.uniform1iv(this.addr,r),Ot(n,r));let o;this.type===s.SAMPLER_CUBE_SHADOW?o=hl(cd):o=fd;for(let a=0;a!==i;++a)t.setTextureCube(e[a]||o,r[a])}function Ax(s,e,t){let n=this.cache,i=e.length,r=cl(t,i);Ft(n,r)||(s.uniform1iv(this.addr,r),Ot(n,r));for(let o=0;o!==i;++o)t.setTexture2DArray(e[o]||hd,r[o])}function Tx(s){switch(s){case 5126:return ax;case 35664:return lx;case 35665:return cx;case 35666:return hx;case 35674:return ux;case 35675:return fx;case 35676:return dx;case 5124:case 35670:return px;case 35667:case 35671:return mx;case 35668:case 35672:return gx;case 35669:case 35673:return _x;case 5125:return xx;case 36294:return yx;case 36295:return vx;case 36296:return Sx;case 35678:case 36198:case 36298:case 36306:case 35682:return Mx;case 35679:case 36299:case 36307:return bx;case 35680:case 36300:case 36308:case 36293:return wx;case 36289:case 36303:case 36311:case 36292:return Ax}}var ah=class{constructor(e,t,n){this.id=e,this.addr=n,this.cache=[],this.type=t.type,this.setValue=ox(t.type)}},lh=class{constructor(e,t,n){this.id=e,this.addr=n,this.cache=[],this.type=t.type,this.size=t.size,this.setValue=Tx(t.type)}},ch=class{constructor(e){this.id=e,this.seq=[],this.map={}}setValue(e,t,n){let i=this.seq;for(let r=0,o=i.length;r!==o;++r){let a=i[r];a.setValue(e,t[a.id],n)}}},rh=/(\w+)(\])?(\[|\.)?/g;function Xf(s,e){s.seq.push(e),s.map[e.id]=e}function Ex(s,e,t){let n=s.name,i=n.length;for(rh.lastIndex=0;;){let r=rh.exec(n),o=rh.lastIndex,a=r[1],l=r[2]==="]",c=r[3];if(l&&(a=a|0),c===void 0||c==="["&&o+2===i){Xf(t,c===void 0?new ah(a,s,e):new lh(a,s,e));break}else{let h=t.map[a];h===void 0&&(h=new ch(a),Xf(t,h)),t=h}}}var qs=class{constructor(e,t){this.seq=[],this.map={};let n=e.getProgramParameter(t,e.ACTIVE_UNIFORMS);for(let o=0;o<n;++o){let a=e.getActiveUniform(t,o),l=e.getUniformLocation(t,a.name);Ex(a,l,this)}let i=[],r=[];for(let o of this.seq)o.type===e.SAMPLER_2D_SHADOW||o.type===e.SAMPLER_CUBE_SHADOW||o.type===e.SAMPLER_2D_ARRAY_SHADOW?i.push(o):r.push(o);i.length>0&&(this.seq=i.concat(r))}setValue(e,t,n,i){let r=this.map[t];r!==void 0&&r.setValue(e,n,i)}setOptional(e,t,n){let i=t[n];i!==void 0&&this.setValue(e,n,i)}static upload(e,t,n,i){for(let r=0,o=t.length;r!==o;++r){let a=t[r],l=n[a.id];l.needsUpdate!==!1&&a.setValue(e,l.value,i)}}static seqWithValue(e,t){let n=[];for(let i=0,r=e.length;i!==r;++i){let o=e[i];o.id in t&&n.push(o)}return n}};function qf(s,e,t){let n=s.createShader(e);return s.shaderSource(n,t),s.compileShader(n),n}var Rx=37297,Cx=0;function Px(s,e){let t=s.split(`
`),n=[],i=Math.max(e-6,0),r=Math.min(e+6,t.length);for(let o=i;o<r;o++){let a=o+1;n.push(`${a===e?">":" "} ${a}: ${t[o]}`)}return n.join(`
`)}var Yf=new Ze;function Ix(s){lt._getMatrix(Yf,lt.workingColorSpace,s);let e=`mat3( ${Yf.elements.map(t=>t.toFixed(4))} )`;switch(lt.getTransfer(s)){case pr:return[e,"LinearTransferOETF"];case pt:return[e,"sRGBTransferOETF"];default:return Xe("WebGLProgram: Unsupported color space: ",s),[e,"LinearTransferOETF"]}}function Zf(s,e,t){let n=s.getShaderParameter(e,s.COMPILE_STATUS),r=(s.getShaderInfoLog(e)||"").trim();if(n&&r==="")return"";let o=/ERROR: 0:(\d+)/.exec(r);if(o){let a=parseInt(o[1]);return t.toUpperCase()+`

`+r+`

`+Px(s.getShaderSource(e),a)}else return r}function Dx(s,e){let t=Ix(e);return[`vec4 ${s}( vec4 value ) {`,`	return ${t[1]}( vec4( ${t[0]} * value.rgb, value.a ) );`,"}"].join(`
`)}var Lx={[Cc]:"Linear",[Pc]:"Reinhard",[Ic]:"Cineon",[Dc]:"ACESFilmic",[Nc]:"AgX",[Uc]:"Neutral",[Lc]:"Custom"};function Nx(s,e){let t=Lx[e];return t===void 0?(Xe("WebGLProgram: Unsupported toneMapping:",e),"vec3 "+s+"( vec3 color ) { return LinearToneMapping( color ); }"):"vec3 "+s+"( vec3 color ) { return "+t+"ToneMapping( color ); }"}var ll=new L;function Ux(){lt.getLuminanceCoefficients(ll);let s=ll.x.toFixed(4),e=ll.y.toFixed(4),t=ll.z.toFixed(4);return["float luminance( const in vec3 rgb ) {",`	const vec3 weights = vec3( ${s}, ${e}, ${t} );`,"	return dot( weights, rgb );","}"].join(`
`)}function Fx(s){return[s.extensionClipCullDistance?"#extension GL_ANGLE_clip_cull_distance : require":"",s.extensionMultiDraw?"#extension GL_ANGLE_multi_draw : require":""].filter(jr).join(`
`)}function Ox(s){let e=[];for(let t in s){let n=s[t];n!==!1&&e.push("#define "+t+" "+n)}return e.join(`
`)}function Bx(s,e){let t={},n=s.getProgramParameter(e,s.ACTIVE_ATTRIBUTES);for(let i=0;i<n;i++){let r=s.getActiveAttrib(e,i),o=r.name,a=1;r.type===s.FLOAT_MAT2&&(a=2),r.type===s.FLOAT_MAT3&&(a=3),r.type===s.FLOAT_MAT4&&(a=4),t[o]={type:r.type,location:s.getAttribLocation(e,o),locationSize:a}}return t}function jr(s){return s!==""}function Jf(s,e){let t=e.numSpotLightShadows+e.numSpotLightMaps-e.numSpotLightShadowsWithMaps;return s.replace(/NUM_SUN_LIGHTS/g,e.numSunLights).replace(/NUM_DIR_LIGHTS/g,e.numDirLights).replace(/NUM_SPOT_LIGHTS/g,e.numSpotLights).replace(/NUM_SPOT_LIGHT_MAPS/g,e.numSpotLightMaps).replace(/NUM_SPOT_LIGHT_COORDS/g,t).replace(/NUM_RECT_AREA_LIGHTS/g,e.numRectAreaLights).replace(/NUM_POINT_LIGHTS/g,e.numPointLights).replace(/NUM_HEMI_LIGHTS/g,e.numHemiLights).replace(/NUM_SUN_LIGHT_SHADOWS/g,e.numSunLightShadows).replace(/NUM_DIR_LIGHT_SHADOWS/g,e.numDirLightShadows).replace(/NUM_SPOT_LIGHT_SHADOWS_WITH_MAPS/g,e.numSpotLightShadowsWithMaps).replace(/NUM_SPOT_LIGHT_SHADOWS/g,e.numSpotLightShadows).replace(/NUM_POINT_LIGHT_SHADOWS/g,e.numPointLightShadows).replace(/NUM_LIGHT_PROBE_GRIDS/g,e.numLightProbeGrids)}function $f(s,e){return s.replace(/NUM_CLIPPING_PLANES/g,e.numClippingPlanes).replace(/UNION_CLIPPING_PLANES/g,e.numClippingPlanes-e.numClipIntersection)}var zx=/^[ \t]*#include +<([\w\d./]+)>/gm;function hh(s){return s.replace(zx,kx)}function kx(s,e){let t=nt[e];if(t===void 0)throw new Error("THREE.WebGLProgram: Can not resolve #include <"+e+">");return hh(t)}var Vx=/#pragma unroll_loop_start\s+for\s*\(\s*int\s+i\s*=\s*(\d+)\s*;\s*i\s*<\s*(\d+)\s*;\s*i\s*\+\+\s*\)\s*{([\s\S]+?)}\s+#pragma unroll_loop_end/g;function Kf(s){return s.replace(Vx,Gx)}function Gx(s,e,t,n){let i="";for(let r=parseInt(e);r<parseInt(t);r++)i+=n.replace(/\[\s*i\s*\]/g,"[ "+r+" ]").replace(/UNROLLED_LOOP_INDEX/g,r);return i}function jf(s){let e=`precision ${s.precision} float;
	precision ${s.precision} int;
	precision ${s.precision} sampler2D;
	precision ${s.precision} samplerCube;
	precision ${s.precision} sampler3D;
	precision ${s.precision} sampler2DArray;
	precision ${s.precision} sampler2DShadow;
	precision ${s.precision} samplerCubeShadow;
	precision ${s.precision} sampler2DArrayShadow;
	precision ${s.precision} isampler2D;
	precision ${s.precision} isampler3D;
	precision ${s.precision} isamplerCube;
	precision ${s.precision} isampler2DArray;
	precision ${s.precision} usampler2D;
	precision ${s.precision} usampler3D;
	precision ${s.precision} usamplerCube;
	precision ${s.precision} usampler2DArray;
	`;return s.precision==="highp"?e+=`
#define HIGH_PRECISION`:s.precision==="mediump"?e+=`
#define MEDIUM_PRECISION`:s.precision==="lowp"&&(e+=`
#define LOW_PRECISION`),e}var Hx={[Gr]:"SHADOWMAP_TYPE_PCF",[Gs]:"SHADOWMAP_TYPE_VSM"};function Wx(s){return Hx[s.shadowMapType]||"SHADOWMAP_TYPE_BASIC"}var Xx={[Ws]:"ENVMAP_MODE_REFRACTION"};function qx(s){return s.envMap===!1?"ENVMAP_MODE_REFLECTION":Xx[s.envMapMode]||"ENVMAP_MODE_REFLECTION"}var Yx={[va]:"ENVMAP_BLENDING_MULTIPLY",[cf]:"ENVMAP_BLENDING_MIX",[hf]:"ENVMAP_BLENDING_ADD"};function Zx(s){return s.envMap===!1?"ENVMAP_BLENDING_NONE":Yx[s.combine]||"ENVMAP_BLENDING_NONE"}function Jx(s,e,t,n){let i=s.getContext(),r=t.defines,o=t.vertexShader,a=t.fragmentShader,l=Wx(t),c=t.envMapPMREM===!0?"ENVMAP_TYPE_PMREM":"ENVMAP_TYPE_CUBE",u=qx(t),h=Zx(t),d=Fx(t),f=Ox(r),m=i.createProgram(),_,p,g=t.glslVersion?"#version "+t.glslVersion+`
`:"";t.isRawShaderMaterial?(_=["#define SHADER_TYPE "+t.shaderType,"#define SHADER_NAME "+t.shaderName,f].filter(jr).join(`
`),_.length>0&&(_+=`
`),p=["#define SHADER_TYPE "+t.shaderType,"#define SHADER_NAME "+t.shaderName,f].filter(jr).join(`
`),p.length>0&&(p+=`
`)):(_=[jf(t),"#define SHADER_TYPE "+t.shaderType,"#define SHADER_NAME "+t.shaderName,f,t.extensionClipCullDistance?"#define USE_CLIP_DISTANCE":"",t.batching?"#define USE_BATCHING":"",t.batchingColor?"#define USE_BATCHING_COLOR":"",t.instancing?"#define USE_INSTANCING":"",t.instancingColor?"#define USE_INSTANCING_COLOR":"",t.instancingMorph?"#define USE_INSTANCING_MORPH":"",t.useFog&&t.fog?"#define USE_FOG":"",t.useFog&&t.fogExp2?"#define FOG_EXP2":"",t.map?"#define USE_MAP":"",t.envMap?"#define USE_ENVMAP":"",t.envMap?"#define "+u:"",t.lightMap?"#define USE_LIGHTMAP":"",t.aoMap?"#define USE_AOMAP":"",t.bumpMap?"#define USE_BUMPMAP":"",t.normalMap?"#define USE_NORMALMAP":"",t.normalMapObjectSpace?"#define USE_NORMALMAP_OBJECTSPACE":"",t.normalMapTangentSpace?"#define USE_NORMALMAP_TANGENTSPACE":"",t.displacementMap?"#define USE_DISPLACEMENTMAP":"",t.emissiveMap?"#define USE_EMISSIVEMAP":"",t.anisotropy?"#define USE_ANISOTROPY":"",t.anisotropyMap?"#define USE_ANISOTROPYMAP":"",t.clearcoatMap?"#define USE_CLEARCOATMAP":"",t.clearcoatRoughnessMap?"#define USE_CLEARCOAT_ROUGHNESSMAP":"",t.clearcoatNormalMap?"#define USE_CLEARCOAT_NORMALMAP":"",t.diffuseRoughnessMap?"#define USE_DIFFUSE_ROUGHNESSMAP":"",t.iridescenceMap?"#define USE_IRIDESCENCEMAP":"",t.iridescenceThicknessMap?"#define USE_IRIDESCENCE_THICKNESSMAP":"",t.specularMap?"#define USE_SPECULARMAP":"",t.specularColorMap?"#define USE_SPECULAR_COLORMAP":"",t.specularIntensityMap?"#define USE_SPECULAR_INTENSITYMAP":"",t.roughnessMap?"#define USE_ROUGHNESSMAP":"",t.metalnessMap?"#define USE_METALNESSMAP":"",t.alphaMap?"#define USE_ALPHAMAP":"",t.alphaHash?"#define USE_ALPHAHASH":"",t.transmission?"#define USE_TRANSMISSION":"",t.transmissionMap?"#define USE_TRANSMISSIONMAP":"",t.thicknessMap?"#define USE_THICKNESSMAP":"",t.sheenColorMap?"#define USE_SHEEN_COLORMAP":"",t.sheenRoughnessMap?"#define USE_SHEEN_ROUGHNESSMAP":"",t.mapUv?"#define MAP_UV "+t.mapUv:"",t.alphaMapUv?"#define ALPHAMAP_UV "+t.alphaMapUv:"",t.lightMapUv?"#define LIGHTMAP_UV "+t.lightMapUv:"",t.aoMapUv?"#define AOMAP_UV "+t.aoMapUv:"",t.emissiveMapUv?"#define EMISSIVEMAP_UV "+t.emissiveMapUv:"",t.bumpMapUv?"#define BUMPMAP_UV "+t.bumpMapUv:"",t.normalMapUv?"#define NORMALMAP_UV "+t.normalMapUv:"",t.displacementMapUv?"#define DISPLACEMENTMAP_UV "+t.displacementMapUv:"",t.metalnessMapUv?"#define METALNESSMAP_UV "+t.metalnessMapUv:"",t.roughnessMapUv?"#define ROUGHNESSMAP_UV "+t.roughnessMapUv:"",t.anisotropyMapUv?"#define ANISOTROPYMAP_UV "+t.anisotropyMapUv:"",t.clearcoatMapUv?"#define CLEARCOATMAP_UV "+t.clearcoatMapUv:"",t.clearcoatNormalMapUv?"#define CLEARCOAT_NORMALMAP_UV "+t.clearcoatNormalMapUv:"",t.clearcoatRoughnessMapUv?"#define CLEARCOAT_ROUGHNESSMAP_UV "+t.clearcoatRoughnessMapUv:"",t.diffuseRoughnessMapUv?"#define DIFFUSE_ROUGHNESSMAP_UV "+t.diffuseRoughnessMapUv:"",t.iridescenceMapUv?"#define IRIDESCENCEMAP_UV "+t.iridescenceMapUv:"",t.iridescenceThicknessMapUv?"#define IRIDESCENCE_THICKNESSMAP_UV "+t.iridescenceThicknessMapUv:"",t.sheenColorMapUv?"#define SHEEN_COLORMAP_UV "+t.sheenColorMapUv:"",t.sheenRoughnessMapUv?"#define SHEEN_ROUGHNESSMAP_UV "+t.sheenRoughnessMapUv:"",t.specularMapUv?"#define SPECULARMAP_UV "+t.specularMapUv:"",t.specularColorMapUv?"#define SPECULAR_COLORMAP_UV "+t.specularColorMapUv:"",t.specularIntensityMapUv?"#define SPECULAR_INTENSITYMAP_UV "+t.specularIntensityMapUv:"",t.transmissionMapUv?"#define TRANSMISSIONMAP_UV "+t.transmissionMapUv:"",t.thicknessMapUv?"#define THICKNESSMAP_UV "+t.thicknessMapUv:"",t.vertexTangents&&t.flatShading===!1?"#define USE_TANGENT":"",t.vertexNormals?"#define HAS_NORMAL":"",t.vertexColors?"#define USE_COLOR":"",t.vertexAlphas?"#define USE_COLOR_ALPHA":"",t.vertexUv1s?"#define USE_UV1":"",t.vertexUv2s?"#define USE_UV2":"",t.vertexUv3s?"#define USE_UV3":"",t.pointsUvs?"#define USE_POINTS_UV":"",t.flatShading?"#define FLAT_SHADED":"",t.skinning?"#define USE_SKINNING":"",t.morphTargets?"#define USE_MORPHTARGETS":"",t.morphNormals&&t.flatShading===!1?"#define USE_MORPHNORMALS":"",t.morphColors?"#define USE_MORPHCOLORS":"",t.morphTargetsCount>0?"#define MORPHTARGETS_TEXTURE_STRIDE "+t.morphTextureStride:"",t.morphTargetsCount>0?"#define MORPHTARGETS_COUNT "+t.morphTargetsCount:"",t.doubleSided?"#define DOUBLE_SIDED":"",t.flipSided?"#define FLIP_SIDED":"",t.shadowMapEnabled?"#define USE_SHADOWMAP":"",t.shadowMapEnabled?"#define "+l:"",t.sizeAttenuation?"#define USE_SIZEATTENUATION":"",t.numLightProbes>0?"#define USE_LIGHT_PROBES":"",t.logarithmicDepthBuffer?"#define USE_LOGARITHMIC_DEPTH_BUFFER":"",t.reversedDepthBuffer?"#define USE_REVERSED_DEPTH_BUFFER":"","uniform mat4 modelMatrix;","uniform mat4 modelViewMatrix;","uniform mat4 projectionMatrix;","uniform mat4 viewMatrix;","uniform mat3 normalMatrix;","uniform vec3 cameraPosition;","uniform bool isOrthographic;","#ifdef USE_INSTANCING","	attribute mat4 instanceMatrix;","#endif","#ifdef USE_INSTANCING_COLOR","	attribute vec3 instanceColor;","#endif","#ifdef USE_INSTANCING_MORPH","	uniform sampler2D morphTexture;","#endif","attribute vec3 position;","attribute vec3 normal;","attribute vec2 uv;","#ifdef USE_UV1","	attribute vec2 uv1;","#endif","#ifdef USE_UV2","	attribute vec2 uv2;","#endif","#ifdef USE_UV3","	attribute vec2 uv3;","#endif","#ifdef USE_TANGENT","	attribute vec4 tangent;","#endif","#if defined( USE_COLOR_ALPHA )","	attribute vec4 color;","#elif defined( USE_COLOR )","	attribute vec3 color;","#endif","#ifdef USE_SKINNING","	attribute vec4 skinIndex;","	attribute vec4 skinWeight;","#endif",`
`].filter(jr).join(`
`),p=[jf(t),"#define SHADER_TYPE "+t.shaderType,"#define SHADER_NAME "+t.shaderName,f,t.useFog&&t.fog?"#define USE_FOG":"",t.useFog&&t.fogExp2?"#define FOG_EXP2":"",t.alphaToCoverage?"#define ALPHA_TO_COVERAGE":"",t.map?"#define USE_MAP":"",t.matcap?"#define USE_MATCAP":"",t.envMap?"#define USE_ENVMAP":"",t.envMap?"#define "+c:"",t.envMap?"#define "+u:"",t.envMap?"#define "+h:"",t.envMapPMREM?"#define ENVMAP_MAX_LOD "+t.envMapMaxLod+".0":"",t.envMapPMREM?"#define ENVMAP_SIZE "+t.envMapSize+".0":"",t.lightMap?"#define USE_LIGHTMAP":"",t.aoMap?"#define USE_AOMAP":"",t.bumpMap?"#define USE_BUMPMAP":"",t.normalMap?"#define USE_NORMALMAP":"",t.normalMapObjectSpace?"#define USE_NORMALMAP_OBJECTSPACE":"",t.normalMapTangentSpace?"#define USE_NORMALMAP_TANGENTSPACE":"",t.packedNormalMap?"#define USE_PACKED_NORMALMAP":"",t.emissiveMap?"#define USE_EMISSIVEMAP":"",t.anisotropy?"#define USE_ANISOTROPY":"",t.anisotropyMap?"#define USE_ANISOTROPYMAP":"",t.clearcoat?"#define USE_CLEARCOAT":"",t.clearcoatMap?"#define USE_CLEARCOATMAP":"",t.clearcoatRoughnessMap?"#define USE_CLEARCOAT_ROUGHNESSMAP":"",t.clearcoatNormalMap?"#define USE_CLEARCOAT_NORMALMAP":"",t.diffuseRoughness?"#define USE_DIFFUSE_ROUGHNESS":"",t.diffuseRoughnessMap?"#define USE_DIFFUSE_ROUGHNESSMAP":"",t.dispersion?"#define USE_DISPERSION":"",t.retroreflection?"#define USE_RETROREFLECTION":"",t.iridescence?"#define USE_IRIDESCENCE":"",t.iridescenceMap?"#define USE_IRIDESCENCEMAP":"",t.iridescenceThicknessMap?"#define USE_IRIDESCENCE_THICKNESSMAP":"",t.specularMap?"#define USE_SPECULARMAP":"",t.specularColorMap?"#define USE_SPECULAR_COLORMAP":"",t.specularIntensityMap?"#define USE_SPECULAR_INTENSITYMAP":"",t.roughnessMap?"#define USE_ROUGHNESSMAP":"",t.metalnessMap?"#define USE_METALNESSMAP":"",t.alphaMap?"#define USE_ALPHAMAP":"",t.alphaTest?"#define USE_ALPHATEST":"",t.alphaHash?"#define USE_ALPHAHASH":"",t.sheen?"#define USE_SHEEN":"",t.sheenColorMap?"#define USE_SHEEN_COLORMAP":"",t.sheenRoughnessMap?"#define USE_SHEEN_ROUGHNESSMAP":"",t.transmission?"#define USE_TRANSMISSION":"",t.transmissionMap?"#define USE_TRANSMISSIONMAP":"",t.thicknessMap?"#define USE_THICKNESSMAP":"",t.vertexTangents&&t.flatShading===!1?"#define USE_TANGENT":"",t.vertexColors||t.instancingColor?"#define USE_COLOR":"",t.vertexAlphas||t.batchingColor?"#define USE_COLOR_ALPHA":"",t.vertexUv1s?"#define USE_UV1":"",t.vertexUv2s?"#define USE_UV2":"",t.vertexUv3s?"#define USE_UV3":"",t.pointsUvs?"#define USE_POINTS_UV":"",t.gradientMap?"#define USE_GRADIENTMAP":"",t.flatShading?"#define FLAT_SHADED":"",t.doubleSided?"#define DOUBLE_SIDED":"",t.flipSided?"#define FLIP_SIDED":"",t.shadowMapEnabled?"#define USE_SHADOWMAP":"",t.shadowMapEnabled?"#define "+l:"",t.premultipliedAlpha?"#define PREMULTIPLIED_ALPHA":"",t.numLightProbes>0?"#define USE_LIGHT_PROBES":"",t.decodeVideoTexture?"#define DECODE_VIDEO_TEXTURE":"",t.decodeVideoTextureEmissive?"#define DECODE_VIDEO_TEXTURE_EMISSIVE":"",t.logarithmicDepthBuffer?"#define USE_LOGARITHMIC_DEPTH_BUFFER":"",t.reversedDepthBuffer?"#define USE_REVERSED_DEPTH_BUFFER":"","uniform mat4 viewMatrix;","uniform vec3 cameraPosition;","uniform bool isOrthographic;",t.toneMapping!==Hn?"#define TONE_MAPPING":"",t.toneMapping!==Hn?nt.tonemapping_pars_fragment:"",t.toneMapping!==Hn?Nx("toneMapping",t.toneMapping):"",t.dithering?"#define DITHERING":"",t.opaque?"#define OPAQUE":"",nt.colorspace_pars_fragment,Dx("linearToOutputTexel",t.outputColorSpace),Ux(),t.useDepthPacking?"#define DEPTH_PACKING "+t.depthPacking:"",`
`].filter(jr).join(`
`)),o=hh(o),o=Jf(o,t),o=$f(o,t),a=hh(a),a=Jf(a,t),a=$f(a,t),o=Kf(o),a=Kf(a),t.isRawShaderMaterial!==!0&&(g=`#version 300 es
`,_=[d,"#define attribute in","#define varying out","#define texture2D texture"].join(`
`)+`
`+_,p=["#define varying in",t.glslVersion===qc?"":"layout(location = 0) out highp vec4 pc_fragColor;",t.glslVersion===qc?"":"#define gl_FragColor pc_fragColor","#define gl_FragDepthEXT gl_FragDepth","#define texture2D texture","#define textureCube texture","#define texture2DProj textureProj","#define texture2DLodEXT textureLod","#define texture2DProjLodEXT textureProjLod","#define textureCubeLodEXT textureLod","#define texture2DGradEXT textureGrad","#define texture2DProjGradEXT textureProjGrad","#define textureCubeGradEXT textureGrad"].join(`
`)+`
`+p);let S=g+_+o,v=g+p+a,x=qf(i,i.VERTEX_SHADER,S),b=qf(i,i.FRAGMENT_SHADER,v);i.attachShader(m,x),i.attachShader(m,b),t.index0AttributeName!==void 0?i.bindAttribLocation(m,0,t.index0AttributeName):t.hasPositionAttribute===!0&&i.bindAttribLocation(m,0,"position"),i.linkProgram(m);function w(E){if(s.debug.checkShaderErrors){let R=i.getProgramInfoLog(m)||"",D=i.getShaderInfoLog(x)||"",k=i.getShaderInfoLog(b)||"",P=R.trim(),O=D.trim(),G=k.trim(),U=!0,ee=!0;if(i.getProgramParameter(m,i.LINK_STATUS)===!1)if(U=!1,typeof s.debug.onShaderError=="function")s.debug.onShaderError(i,m,x,b);else{let W=Zf(i,x,"vertex"),K=Zf(i,b,"fragment");Ye("WebGLProgram: Shader Error "+i.getError()+" - VALIDATE_STATUS "+i.getProgramParameter(m,i.VALIDATE_STATUS)+`

Material Name: `+E.name+`
Material Type: `+E.type+`

Program Info Log: `+P+`
`+W+`
`+K)}else P!==""?Xe("WebGLProgram: Program Info Log:",P):(O===""||G==="")&&(ee=!1);ee&&(E.diagnostics={runnable:U,programLog:P,vertexShader:{log:O,prefix:_},fragmentShader:{log:G,prefix:p}})}i.deleteShader(x),i.deleteShader(b),y=new qs(i,m),M=Bx(i,m)}let y;this.getUniforms=function(){return y===void 0&&w(this),y};let M;this.getAttributes=function(){return M===void 0&&w(this),M};let C=t.rendererExtensionParallelShaderCompile===!1;return this.isReady=function(){return C===!1&&(C=i.getProgramParameter(m,Rx)),C},this.destroy=function(){n.releaseStatesOfProgram(this),i.deleteProgram(m),this.program=void 0},this.type=t.shaderType,this.name=t.shaderName,this.id=Cx++,this.cacheKey=e,this.usedTimes=1,this.program=m,this.vertexShader=x,this.fragmentShader=b,this}var $x=0,uh=class{constructor(){this.shaderCache=new Map,this.materialCache=new Map}update(e,t,n){let i=this._getShaderCacheForMaterial(e);return i.has(t)===!1&&(i.add(t),t.usedTimes++),i.has(n)===!1&&(i.add(n),n.usedTimes++),this}remove(e){let t=this.materialCache.get(e);for(let n of t)n.usedTimes--,n.usedTimes===0&&this.shaderCache.delete(n.code);return this.materialCache.delete(e),this}getVertexShaderStage(e){return this._getShaderStage(e.vertexShader)}getFragmentShaderStage(e){return this._getShaderStage(e.fragmentShader)}dispose(){this.shaderCache.clear(),this.materialCache.clear()}_getShaderCacheForMaterial(e){let t=this.materialCache,n=t.get(e);return n===void 0&&(n=new Set,t.set(e,n)),n}_getShaderStage(e){let t=this.shaderCache,n=t.get(e);return n===void 0&&(n=new fh(e),t.set(e,n)),n}},fh=class{constructor(e){this.id=$x++,this.code=e,this.usedTimes=0}};function Kx(s){return s===Ci||s===Zr||s===Jr}function jx(s,e,t,n,i,r){let o=new Cs,a=new uh,l=new Set,c=[],u=new Map,h=n.logarithmicDepthBuffer,d={MeshDepthMaterial:"depth",MeshDistanceMaterial:"distance",MeshNormalMaterial:"normal",MeshBasicMaterial:"basic",MeshLambertMaterial:"lambert",MeshPhongMaterial:"phong",MeshToonMaterial:"toon",MeshStandardMaterial:"physical",MeshPhysicalMaterial:"physical",MeshMatcapMaterial:"matcap",LineBasicMaterial:"basic",LineDashedMaterial:"dashed",PointsMaterial:"points",ShadowMaterial:"shadow",SpriteMaterial:"sprite"};function f(y){return l.add(y),y===0?"uv":`uv${y}`}function m(y,M,C,E,R,D){let k=E.fog,P=R.geometry,O=y.isMeshStandardMaterial||y.isMeshLambertMaterial||y.isMeshPhongMaterial?E.environment:null,G=y.isMeshStandardMaterial||y.isMeshLambertMaterial&&!y.envMap||y.isMeshPhongMaterial&&!y.envMap,U=e.get(y.envMap||O,G),ee=d[y.type],W=n.precision;y.precision!==null&&(W=n.getMaxPrecision(y.precision),W!==y.precision&&Xe("WebGLProgram.getParameters:",y.precision,"not supported, using",W,"instead."));let K=P.morphAttributes.position||P.morphAttributes.normal||P.morphAttributes.color,$=K!==void 0?K.length:0,he=0;P.morphAttributes.position!==void 0&&(he=1),P.morphAttributes.normal!==void 0&&(he=2),P.morphAttributes.color!==void 0&&(he=3);let de,q,ie,ce;if(ee){let Pe=Xn[ee];de=Pe.vertexShader,q=Pe.fragmentShader}else{de=y.vertexShader,q=y.fragmentShader;let Pe=a.getVertexShaderStage(y),mt=a.getFragmentShaderStage(y);a.update(y,Pe,mt),ie=Pe.id,ce=mt.id}let Ce=s.getRenderTarget(),ue=s.state.buffers.depth.getReversed(),H=R.isInstancedMesh===!0,se=R.isBatchedMesh===!0,fe=!!y.map,Le=!!y.matcap,V=!!U,N=V&&U.isPMREMTexture===!0,X=!!y.aoMap,j=!!y.lightMap,ge=!!y.bumpMap&&y.wireframe===!1,le=!!y.normalMap,Re=!!y.displacementMap,xe=!!y.emissiveMap,Ue=!!y.metalnessMap,Je=!!y.roughnessMap,Qe=y.anisotropy>0,B=y.clearcoat>0,xt=y.diffuseRoughness>0,je=y.dispersion>0,rt=y.retroreflectivity>0,Me=y.iridescence>0,yt=y.sheen>0,He=y.transmission>0,qe=Qe&&!!y.anisotropyMap,I=B&&!!y.clearcoatMap,A=B&&!!y.clearcoatNormalMap,Z=B&&!!y.clearcoatRoughnessMap,me=xt&&!!y.diffuseRoughnessMap,ae=Me&&!!y.iridescenceMap,ve=Me&&!!y.iridescenceThicknessMap,Te=yt&&!!y.sheenColorMap,ne=yt&&!!y.sheenRoughnessMap,re=!!y.specularMap,be=!!y.specularColorMap,F=!!y.specularIntensityMap,oe=He&&!!y.transmissionMap,te=He&&!!y.thicknessMap,we=!!y.gradientMap,ye=!!y.alphaMap,Se=y.alphaTest>0,Ge=!!y.alphaHash,tt=!!y.extensions,_e=Hn;y.toneMapped&&(Ce===null||Ce.isXRRenderTarget===!0)&&(_e=s.toneMapping);let We=N?U.isCompressedCubeTexture?U.image[0].mipmaps:U.mipmaps:null,et={shaderID:ee,shaderType:y.type,shaderName:y.name,vertexShader:de,fragmentShader:q,defines:y.defines,customVertexShaderID:ie,customFragmentShaderID:ce,isRawShaderMaterial:y.isRawShaderMaterial===!0,glslVersion:y.glslVersion,precision:W,batching:se,batchingColor:se&&R._colorsTexture!==null,instancing:H,instancingColor:H&&R.instanceColor!==null,instancingMorph:H&&R.morphTexture!==null,outputColorSpace:Ce===null?s.outputColorSpace:Ce.isXRRenderTarget===!0?Ce.texture.colorSpace:lt.workingColorSpace,alphaToCoverage:!!y.alphaToCoverage,map:fe,matcap:Le,envMap:V,envMapMode:V&&U.mapping,envMapPMREM:N,envMapMaxLod:N?We.length-1:null,envMapSize:N?We[0].width:null,aoMap:X,lightMap:j,bumpMap:ge,normalMap:le,displacementMap:Re,emissiveMap:xe,normalMapObjectSpace:le&&y.normalMapType===mf,normalMapTangentSpace:le&&y.normalMapType===$r,packedNormalMap:le&&y.normalMapType===$r&&Kx(y.normalMap.format),metalnessMap:Ue,roughnessMap:Je,anisotropy:Qe,anisotropyMap:qe,clearcoat:B,clearcoatMap:I,clearcoatNormalMap:A,clearcoatRoughnessMap:Z,diffuseRoughness:xt,diffuseRoughnessMap:me,dispersion:je,retroreflection:rt,iridescence:Me,iridescenceMap:ae,iridescenceThicknessMap:ve,sheen:yt,sheenColorMap:Te,sheenRoughnessMap:ne,specularMap:re,specularColorMap:be,specularIntensityMap:F,transmission:He,transmissionMap:oe,thicknessMap:te,gradientMap:we,opaque:y.transparent===!1&&y.blending===Hs&&y.alphaToCoverage===!1,alphaMap:ye,alphaTest:Se,alphaHash:Ge,combine:y.combine,mapUv:fe&&f(y.map.channel),aoMapUv:X&&f(y.aoMap.channel),lightMapUv:j&&f(y.lightMap.channel),bumpMapUv:ge&&f(y.bumpMap.channel),normalMapUv:le&&f(y.normalMap.channel),displacementMapUv:Re&&f(y.displacementMap.channel),emissiveMapUv:xe&&f(y.emissiveMap.channel),metalnessMapUv:Ue&&f(y.metalnessMap.channel),roughnessMapUv:Je&&f(y.roughnessMap.channel),anisotropyMapUv:qe&&f(y.anisotropyMap.channel),clearcoatMapUv:I&&f(y.clearcoatMap.channel),clearcoatNormalMapUv:A&&f(y.clearcoatNormalMap.channel),clearcoatRoughnessMapUv:Z&&f(y.clearcoatRoughnessMap.channel),diffuseRoughnessMapUv:me&&f(y.diffuseRoughnessMap.channel),iridescenceMapUv:ae&&f(y.iridescenceMap.channel),iridescenceThicknessMapUv:ve&&f(y.iridescenceThicknessMap.channel),sheenColorMapUv:Te&&f(y.sheenColorMap.channel),sheenRoughnessMapUv:ne&&f(y.sheenRoughnessMap.channel),specularMapUv:re&&f(y.specularMap.channel),specularColorMapUv:be&&f(y.specularColorMap.channel),specularIntensityMapUv:F&&f(y.specularIntensityMap.channel),transmissionMapUv:oe&&f(y.transmissionMap.channel),thicknessMapUv:te&&f(y.thicknessMap.channel),alphaMapUv:ye&&f(y.alphaMap.channel),vertexTangents:!!P.attributes.tangent&&(le||Qe),vertexNormals:!!P.attributes.normal,vertexColors:y.vertexColors,vertexAlphas:y.vertexColors===!0&&!!P.attributes.color&&P.attributes.color.itemSize===4,pointsUvs:R.isPoints===!0&&!!P.attributes.uv&&(fe||ye),fog:!!k,useFog:y.fog===!0,fogExp2:!!k&&k.isFogExp2,flatShading:y.wireframe===!1&&(y.flatShading===!0||P.attributes.normal===void 0&&le===!1&&(y.isMeshLambertMaterial||y.isMeshPhongMaterial||y.isMeshStandardMaterial||y.isMeshPhysicalMaterial)),sizeAttenuation:y.sizeAttenuation===!0,logarithmicDepthBuffer:h,reversedDepthBuffer:ue,skinning:R.isSkinnedMesh===!0,hasPositionAttribute:P.attributes.position!==void 0,morphTargets:P.morphAttributes.position!==void 0,morphNormals:P.morphAttributes.normal!==void 0,morphColors:P.morphAttributes.color!==void 0,morphTargetsCount:$,morphTextureStride:he,numSunLights:M.sun.length,numDirLights:M.directional.length,numPointLights:M.point.length,numSpotLights:M.spot.length,numSpotLightMaps:M.spotLightMap.length,numRectAreaLights:M.rectArea.length,numHemiLights:M.hemi.length,numSunLightShadows:M.sunShadowMap.length,numDirLightShadows:M.directionalShadowMap.length,numPointLightShadows:M.pointShadowMap.length,numSpotLightShadows:M.spotShadowMap.length,numSpotLightShadowsWithMaps:M.numSpotLightShadowsWithMaps,numLightProbes:M.numLightProbes,numLightProbeGrids:D.filter(Pe=>Pe.texture!==null).length,numClippingPlanes:r.numPlanes,numClipIntersection:r.numIntersection,dithering:y.dithering,shadowMapEnabled:s.shadowMap.enabled&&C.length>0,shadowMapType:s.shadowMap.type,toneMapping:_e,decodeVideoTexture:fe&&y.map.isVideoTexture===!0&&lt.getTransfer(y.map.colorSpace)===pt,decodeVideoTextureEmissive:xe&&y.emissiveMap.isVideoTexture===!0&&lt.getTransfer(y.emissiveMap.colorSpace)===pt,premultipliedAlpha:y.premultipliedAlpha,doubleSided:y.side===Gn,flipSided:y.side===Xt,useDepthPacking:y.depthPacking>=0,depthPacking:y.depthPacking||0,index0AttributeName:y.index0AttributeName,extensionClipCullDistance:tt&&y.extensions.clipCullDistance===!0&&t.has("WEBGL_clip_cull_distance"),extensionMultiDraw:(tt&&y.extensions.multiDraw===!0||se)&&t.has("WEBGL_multi_draw"),rendererExtensionParallelShaderCompile:t.has("KHR_parallel_shader_compile"),customProgramCacheKey:y.customProgramCacheKey()};return et.vertexUv1s=l.has(1),et.vertexUv2s=l.has(2),et.vertexUv3s=l.has(3),l.clear(),et}function _(y){let M=[];if(y.shaderID?M.push(y.shaderID):(M.push(y.customVertexShaderID),M.push(y.customFragmentShaderID)),y.defines!==void 0)for(let C in y.defines)M.push(C),M.push(y.defines[C]);return y.isRawShaderMaterial===!1&&(p(M,y),g(M,y),M.push(s.outputColorSpace)),M.push(y.customProgramCacheKey),M.join()}function p(y,M){y.push(M.precision),y.push(M.outputColorSpace),y.push(M.envMapMode),y.push(M.envMapPMREM),y.push(M.envMapMaxLod),y.push(M.envMapSize),y.push(M.mapUv),y.push(M.alphaMapUv),y.push(M.lightMapUv),y.push(M.aoMapUv),y.push(M.bumpMapUv),y.push(M.normalMapUv),y.push(M.displacementMapUv),y.push(M.emissiveMapUv),y.push(M.metalnessMapUv),y.push(M.roughnessMapUv),y.push(M.anisotropyMapUv),y.push(M.clearcoatMapUv),y.push(M.clearcoatNormalMapUv),y.push(M.clearcoatRoughnessMapUv),y.push(M.diffuseRoughnessMapUv),y.push(M.iridescenceMapUv),y.push(M.iridescenceThicknessMapUv),y.push(M.sheenColorMapUv),y.push(M.sheenRoughnessMapUv),y.push(M.specularMapUv),y.push(M.specularColorMapUv),y.push(M.specularIntensityMapUv),y.push(M.transmissionMapUv),y.push(M.thicknessMapUv),y.push(M.combine),y.push(M.fogExp2),y.push(M.sizeAttenuation),y.push(M.morphTargetsCount),y.push(M.numSunLights),y.push(M.numDirLights),y.push(M.numPointLights),y.push(M.numSpotLights),y.push(M.numSpotLightMaps),y.push(M.numHemiLights),y.push(M.numRectAreaLights),y.push(M.numSunLightShadows),y.push(M.numDirLightShadows),y.push(M.numPointLightShadows),y.push(M.numSpotLightShadows),y.push(M.numSpotLightShadowsWithMaps),y.push(M.numLightProbes),y.push(M.numLightProbeGrids),y.push(M.shadowMapType),y.push(M.toneMapping),y.push(M.numClippingPlanes),y.push(M.numClipIntersection),y.push(M.depthPacking)}function g(y,M){o.disableAll(),M.instancing&&o.enable(0),M.instancingColor&&o.enable(1),M.instancingMorph&&o.enable(2),M.matcap&&o.enable(3),M.envMap&&o.enable(4),M.normalMapObjectSpace&&o.enable(5),M.normalMapTangentSpace&&o.enable(6),M.clearcoat&&o.enable(7),M.iridescence&&o.enable(8),M.alphaTest&&o.enable(9),M.vertexColors&&o.enable(10),M.vertexAlphas&&o.enable(11),M.vertexUv1s&&o.enable(12),M.vertexUv2s&&o.enable(13),M.vertexUv3s&&o.enable(14),M.vertexTangents&&o.enable(15),M.anisotropy&&o.enable(16),M.alphaHash&&o.enable(17),M.batching&&o.enable(18),M.dispersion&&o.enable(19),M.retroreflection&&o.enable(24),M.diffuseRoughness&&o.enable(25),M.batchingColor&&o.enable(20),M.gradientMap&&o.enable(21),M.packedNormalMap&&o.enable(22),M.vertexNormals&&o.enable(23),y.push(o.mask),o.disableAll(),M.fog&&o.enable(0),M.useFog&&o.enable(1),M.flatShading&&o.enable(2),M.logarithmicDepthBuffer&&o.enable(3),M.reversedDepthBuffer&&o.enable(4),M.skinning&&o.enable(5),M.morphTargets&&o.enable(6),M.morphNormals&&o.enable(7),M.morphColors&&o.enable(8),M.premultipliedAlpha&&o.enable(9),M.shadowMapEnabled&&o.enable(10),M.doubleSided&&o.enable(11),M.flipSided&&o.enable(12),M.useDepthPacking&&o.enable(13),M.dithering&&o.enable(14),M.transmission&&o.enable(15),M.sheen&&o.enable(16),M.opaque&&o.enable(17),M.pointsUvs&&o.enable(18),M.decodeVideoTexture&&o.enable(19),M.decodeVideoTextureEmissive&&o.enable(20),M.alphaToCoverage&&o.enable(21),M.numLightProbeGrids>0&&o.enable(22),M.hasPositionAttribute&&o.enable(23),y.push(o.mask)}function S(y){let M=d[y.type],C;if(M){let E=Xn[M];C=Lf.clone(E.uniforms)}else C=y.uniforms;return C}function v(y,M){let C=u.get(M);return C!==void 0?++C.usedTimes:(C=new Jx(s,M,y,i),c.push(C),u.set(M,C)),C}function x(y){if(--y.usedTimes===0){let M=c.indexOf(y);c[M]=c[c.length-1],c.pop(),u.delete(y.cacheKey),y.destroy()}}function b(y){a.remove(y)}function w(){a.dispose()}return{getParameters:m,getProgramCacheKey:_,getUniforms:S,acquireProgram:v,releaseProgram:x,releaseShaderCache:b,programs:c,dispose:w}}function Qx(){let s=new WeakMap;function e(o){return s.has(o)}function t(o){let a=s.get(o);return a===void 0&&(a={},s.set(o,a)),a}function n(o){s.delete(o)}function i(o,a,l){s.get(o)[a]=l}function r(){s=new WeakMap}return{has:e,get:t,remove:n,update:i,dispose:r}}function ey(s,e){return s.groupOrder!==e.groupOrder?s.groupOrder-e.groupOrder:s.renderOrder!==e.renderOrder?s.renderOrder-e.renderOrder:s.material.id!==e.material.id?s.material.id-e.material.id:s.materialVariant!==e.materialVariant?s.materialVariant-e.materialVariant:s.z!==e.z?s.z-e.z:s.id-e.id}function Qf(s,e){return s.groupOrder!==e.groupOrder?s.groupOrder-e.groupOrder:s.renderOrder!==e.renderOrder?s.renderOrder-e.renderOrder:s.z!==e.z?e.z-s.z:s.id-e.id}function ed(){let s=[],e=0,t=[],n=[],i=[];function r(){e=0,t.length=0,n.length=0,i.length=0}function o(d){let f=0;return d.isInstancedMesh&&(f+=2),d.isSkinnedMesh&&(f+=1),f}function a(d,f,m,_,p,g){let S=s[e];return S===void 0?(S={id:d.id,object:d,geometry:f,material:m,materialVariant:o(d),groupOrder:_,renderOrder:d.renderOrder,z:p,group:g},s[e]=S):(S.id=d.id,S.object=d,S.geometry=f,S.material=m,S.materialVariant=o(d),S.groupOrder=_,S.renderOrder=d.renderOrder,S.z=p,S.group=g),e++,S}function l(d,f,m,_,p,g,S){S.reversedDepth===!0&&(p=-p);let v=a(d,f,m,_,p,g);m.transmission>0?n.push(v):m.transparent===!0?i.push(v):t.push(v)}function c(d,f,m,_,p,g){let S=a(d,f,m,_,p,g);m.transmission>0?n.unshift(S):m.transparent===!0?i.unshift(S):t.unshift(S)}function u(d,f){t.length>1&&t.sort(d||ey),n.length>1&&n.sort(f||Qf),i.length>1&&i.sort(f||Qf)}function h(){for(let d=e,f=s.length;d<f;d++){let m=s[d];if(m.id===null)break;m.id=null,m.object=null,m.geometry=null,m.material=null,m.group=null}}return{opaque:t,transmissive:n,transparent:i,init:r,push:l,unshift:c,finish:h,sort:u}}function ty(){let s=new WeakMap;function e(n,i){let r=s.get(n),o;return r===void 0?(o=new ed,s.set(n,[o])):i>=r.length?(o=new ed,r.push(o)):o=r[i],o}function t(){s=new WeakMap}return{get:e,dispose:t}}function ny(){let s={};return{get:function(e){if(s[e.id]!==void 0)return s[e.id];let t;switch(e.type){case"SunLight":case"DirectionalLight":t={direction:new L,color:new Ke};break;case"SpotLight":t={position:new L,direction:new L,color:new Ke,distance:0,coneCos:0,penumbraCos:0,decay:0};break;case"PointLight":t={position:new L,color:new Ke,distance:0,decay:0};break;case"HemisphereLight":t={direction:new L,skyColor:new Ke,groundColor:new Ke};break;case"RectAreaLight":t={color:new Ke,position:new L,halfWidth:new L,halfHeight:new L};break}return s[e.id]=t,t}}}function iy(){let s={};return{get:function(e){if(s[e.id]!==void 0)return s[e.id];let t;switch(e.type){case"SunLight":case"DirectionalLight":t={shadowIntensity:1,shadowBias:0,shadowNormalBias:0,shadowRadius:1,shadowMapSize:new pe};break;case"SpotLight":t={shadowIntensity:1,shadowBias:0,shadowNormalBias:0,shadowRadius:1,shadowMapSize:new pe};break;case"PointLight":t={shadowIntensity:1,shadowBias:0,shadowNormalBias:0,shadowRadius:1,shadowMapSize:new pe,shadowCameraNear:1,shadowCameraFar:1e3};break}return s[e.id]=t,t}}}var sy=0;function ry(s,e){return(e.castShadow?2:0)-(s.castShadow?2:0)+(e.map?1:0)-(s.map?1:0)}function oy(s){let e=new ny,t=iy(),n={version:0,hash:{sunLength:-1,directionalLength:-1,pointLength:-1,spotLength:-1,rectAreaLength:-1,hemiLength:-1,numSunShadows:-1,numDirectionalShadows:-1,numPointShadows:-1,numSpotShadows:-1,numSpotMaps:-1,numLightProbes:-1},ambient:[0,0,0],probe:[],sun:[],sunShadow:[],sunShadowMap:[],sunShadowMatrix:[],sunShadowCascade:[],directional:[],directionalShadow:[],directionalShadowMap:[],directionalShadowMatrix:[],spot:[],spotLightMap:[],spotShadow:[],spotShadowMap:[],spotLightMatrix:[],rectArea:[],rectAreaLTC1:null,rectAreaLTC2:null,point:[],pointShadow:[],pointShadowMap:[],pointShadowMatrix:[],hemi:[],numSpotLightShadowsWithMaps:0,numLightProbes:0};for(let c=0;c<9;c++)n.probe.push(new L);let i=new L,r=new ze,o=new ze;function a(c){let u=0,h=0,d=0;for(let D=0;D<9;D++)n.probe[D].set(0,0,0);let f=0,m=0,_=0,p=0,g=0,S=0,v=0,x=0,b=0,w=0,y=0,M=0,C=0,E=0;c.sort(ry);for(let D=0,k=c.length;D<k;D++){let P=c[D],O=P.color,G=P.intensity,U=P.distance,ee=null;if(P.shadow&&P.shadow.map&&(P.shadow.map.texture.format===Ci?ee=P.shadow.map.texture:ee=P.shadow.map.depthTexture||P.shadow.map.texture),P.isAmbientLight)u+=O.r*G,h+=O.g*G,d+=O.b*G;else if(P.isLightProbe){for(let W=0;W<9;W++)n.probe[W].addScaledVector(P.sh.coefficients[W],G);E++}else if(P.isSunLight){let W=e.get(P);if(W.color.copy(P.color).multiplyScalar(P.intensity),P.castShadow){let K=P.shadow,$=t.get(P);$.shadowIntensity=K.intensity,$.shadowBias=K.bias,$.shadowNormalBias=K.normalBias,$.shadowRadius=K.radius,$.shadowMapSize.copy(K.mapSize).multiply(K.getFrameExtents()),n.sunShadow[m]=$,n.sunShadowMap[m]=ee;let he=K.getViewportCount();for(let de=0;de<he;de++)n.sunShadowMatrix[_+de]=K.getMatrix(de),n.sunShadowCascade[_+de]=K._cascadeData[de];_+=he,m++}n.sun[f]=W,f++}else if(P.isDirectionalLight){let W=e.get(P);if(W.color.copy(P.color).multiplyScalar(P.intensity),P.castShadow){let K=P.shadow,$=t.get(P);$.shadowIntensity=K.intensity,$.shadowBias=K.bias,$.shadowNormalBias=K.normalBias,$.shadowRadius=K.radius,$.shadowMapSize=K.mapSize,n.directionalShadow[p]=$,n.directionalShadowMap[p]=ee,n.directionalShadowMatrix[p]=P.shadow.matrix,b++}n.directional[p]=W,p++}else if(P.isSpotLight){let W=e.get(P);W.position.setFromMatrixPosition(P.matrixWorld),W.color.copy(O).multiplyScalar(G),W.distance=U,W.coneCos=Math.cos(P.angle),W.penumbraCos=Math.cos(P.angle*(1-P.penumbra)),W.decay=P.decay,n.spot[S]=W;let K=P.shadow;if(P.map&&(n.spotLightMap[M]=P.map,M++,K.updateMatrices(P),P.castShadow&&C++),n.spotLightMatrix[S]=K.matrix,P.castShadow){let $=t.get(P);$.shadowIntensity=K.intensity,$.shadowBias=K.bias,$.shadowNormalBias=K.normalBias,$.shadowRadius=K.radius,$.shadowMapSize=K.mapSize,n.spotShadow[S]=$,n.spotShadowMap[S]=ee,y++}S++}else if(P.isRectAreaLight){let W=e.get(P);W.color.copy(O).multiplyScalar(G),W.halfWidth.set(P.width*.5,0,0),W.halfHeight.set(0,P.height*.5,0),n.rectArea[v]=W,v++}else if(P.isPointLight){let W=e.get(P);if(W.color.copy(P.color).multiplyScalar(P.intensity),W.distance=P.distance,W.decay=P.decay,P.castShadow){let K=P.shadow,$=t.get(P);$.shadowIntensity=K.intensity,$.shadowBias=K.bias,$.shadowNormalBias=K.normalBias,$.shadowRadius=K.radius,$.shadowMapSize=K.mapSize,$.shadowCameraNear=K.camera.near,$.shadowCameraFar=K.camera.far,n.pointShadow[g]=$,n.pointShadowMap[g]=ee,n.pointShadowMatrix[g]=P.shadow.matrix,w++}n.point[g]=W,g++}else if(P.isHemisphereLight){let W=e.get(P);W.skyColor.copy(P.color).multiplyScalar(G),W.groundColor.copy(P.groundColor).multiplyScalar(G),n.hemi[x]=W,x++}}v>0&&(s.has("OES_texture_float_linear")===!0?(n.rectAreaLTC1=Ie.LTC_FLOAT_1,n.rectAreaLTC2=Ie.LTC_FLOAT_2):(n.rectAreaLTC1=Ie.LTC_HALF_1,n.rectAreaLTC2=Ie.LTC_HALF_2)),n.ambient[0]=u,n.ambient[1]=h,n.ambient[2]=d;let R=n.hash;(R.sunLength!==f||R.directionalLength!==p||R.pointLength!==g||R.spotLength!==S||R.rectAreaLength!==v||R.hemiLength!==x||R.numSunShadows!==m||R.numDirectionalShadows!==b||R.numPointShadows!==w||R.numSpotShadows!==y||R.numSpotMaps!==M||R.numLightProbes!==E)&&(n.sun.length=f,n.directional.length=p,n.spot.length=S,n.rectArea.length=v,n.point.length=g,n.hemi.length=x,n.sunShadow.length=m,n.sunShadowMap.length=m,n.sunShadowMatrix.length=_,n.sunShadowCascade.length=_,n.directionalShadow.length=b,n.directionalShadowMap.length=b,n.directionalShadowMatrix.length=b,n.pointShadow.length=w,n.pointShadowMap.length=w,n.pointShadowMatrix.length=w,n.spotShadow.length=y,n.spotShadowMap.length=y,n.spotLightMatrix.length=y+M-C,n.spotLightMap.length=M,n.numSpotLightShadowsWithMaps=C,n.numLightProbes=E,R.sunLength=f,R.directionalLength=p,R.pointLength=g,R.spotLength=S,R.rectAreaLength=v,R.hemiLength=x,R.numSunShadows=m,R.numDirectionalShadows=b,R.numPointShadows=w,R.numSpotShadows=y,R.numSpotMaps=M,R.numLightProbes=E,n.version=sy++)}function l(c,u){let h=0,d=0,f=0,m=0,_=0,p=0,g=u.matrixWorldInverse;for(let S=0,v=c.length;S<v;S++){let x=c[S];if(x.isSunLight){let b=n.sun[h];b.direction.setFromMatrixPosition(x.matrixWorld),b.direction.transformDirection(g),h++}else if(x.isDirectionalLight){let b=n.directional[d];b.direction.setFromMatrixPosition(x.matrixWorld),i.setFromMatrixPosition(x.target.matrixWorld),b.direction.sub(i),b.direction.transformDirection(g),d++}else if(x.isSpotLight){let b=n.spot[m];b.position.setFromMatrixPosition(x.matrixWorld),b.position.applyMatrix4(g),b.direction.setFromMatrixPosition(x.matrixWorld),i.setFromMatrixPosition(x.target.matrixWorld),b.direction.sub(i),b.direction.transformDirection(g),m++}else if(x.isRectAreaLight){let b=n.rectArea[_];b.position.setFromMatrixPosition(x.matrixWorld),b.position.applyMatrix4(g),o.identity(),r.copy(x.matrixWorld),r.premultiply(g),o.extractRotation(r),b.halfWidth.set(x.width*.5,0,0),b.halfHeight.set(0,x.height*.5,0),b.halfWidth.applyMatrix4(o),b.halfHeight.applyMatrix4(o),_++}else if(x.isPointLight){let b=n.point[f];b.position.setFromMatrixPosition(x.matrixWorld),b.position.applyMatrix4(g),f++}else if(x.isHemisphereLight){let b=n.hemi[p];b.direction.setFromMatrixPosition(x.matrixWorld),b.direction.transformDirection(g),p++}}}return{setup:a,setupView:l,state:n}}function td(s){let e=new oy(s),t=[],n=[],i=[];function r(h){u.camera=h,t.length=0,n.length=0,i.length=0}function o(h){h.isLightProbeGrid?i.push(h):t.push(h)}function a(h){n.push(h)}function l(){e.setup(t),i.sort((h,d)=>h.priority-d.priority||h.id-d.id)}function c(h){e.setupView(t,h)}let u={lightsArray:t,shadowsArray:n,lightProbeGridArray:i,camera:null,lights:e,transmissionRenderTarget:{},textureUnits:0};return{init:r,state:u,setupLights:l,setupLightsView:c,pushLight:o,pushShadow:a}}function ay(s){let e=new WeakMap;function t(i,r=0){let o=e.get(i),a;return o===void 0?(a=new td(s),e.set(i,[a])):r>=o.length?(a=new td(s),o.push(a)):a=o[r],a}function n(){e=new WeakMap}return{get:t,dispose:n}}var ly=`void main() {
	gl_Position = vec4( position, 1.0 );
}`,cy=`uniform sampler2D shadow_pass;
uniform vec2 resolution;
uniform float radius;
void main() {
	const float samples = float( VSM_SAMPLES );
	float mean = 0.0;
	float squared_mean = 0.0;
	float uvStride = samples <= 1.0 ? 0.0 : 2.0 / ( samples - 1.0 );
	float uvStart = samples <= 1.0 ? 0.0 : - 1.0;
	for ( float i = 0.0; i < samples; i ++ ) {
		float uvOffset = uvStart + i * uvStride;
		#ifdef HORIZONTAL_PASS
			vec2 distribution = texture2D( shadow_pass, ( gl_FragCoord.xy + vec2( uvOffset, 0.0 ) * radius ) / resolution ).rg;
			mean += distribution.x;
			squared_mean += distribution.y * distribution.y + distribution.x * distribution.x;
		#else
			float depth = texture2D( shadow_pass, ( gl_FragCoord.xy + vec2( 0.0, uvOffset ) * radius ) / resolution ).r;
			mean += depth;
			squared_mean += depth * depth;
		#endif
	}
	mean = mean / samples;
	squared_mean = squared_mean / samples;
	float std_dev = sqrt( max( 0.0, squared_mean - mean * mean ) );
	gl_FragColor = vec4( mean, std_dev, 0.0, 1.0 );
}`,hy=[new L(1,0,0),new L(-1,0,0),new L(0,1,0),new L(0,-1,0),new L(0,0,1),new L(0,0,-1)],uy=[new L(0,-1,0),new L(0,-1,0),new L(0,0,1),new L(0,0,-1),new L(0,-1,0),new L(0,-1,0)],nd=new ze,Kr=new L,oh=new L;function fy(s,e,t){let n=new wi,i=new pe,r=new pe,o=new _t,a=new ha,l=new ua,c={},u=t.maxTextureSize,h={[Ri]:Xt,[Xt]:Ri,[Gn]:Gn},d=new Vn({defines:{VSM_SAMPLES:8},uniforms:{shadow_pass:{value:null},resolution:{value:new pe},radius:{value:4}},vertexShader:ly,fragmentShader:cy}),f=d.clone();f.defines.HORIZONTAL_PASS=1;let m=new Lt;m.setAttribute("position",new gt(new Float32Array([-1,-1,.5,3,-1,.5,-1,3,.5]),3));let _=new ke(m,d),p=this;this.enabled=!1,this.autoUpdate=!0,this.needsUpdate=!1,this.type=Gr;let g=this.type;this.render=function(w,y,M){if(p.enabled===!1||p.autoUpdate===!1&&p.needsUpdate===!1||w.length===0)return;this.type===Hu&&(Xe("WebGLShadowMap: PCFSoftShadowMap has been removed. Using PCFShadowMap instead."),this.type=Gr);let C=s.getRenderTarget(),E=s.getActiveCubeFace(),R=s.getActiveMipmapLevel(),D=s.state;D.setBlending($i),D.buffers.depth.getReversed()===!0?D.buffers.color.setClear(0,0,0,0):D.buffers.color.setClear(1,1,1,1),D.buffers.depth.setTest(!0),D.setScissorTest(!1);let k=g!==this.type;k&&y.traverse(function(P){P.material&&(Array.isArray(P.material)?P.material.forEach(O=>O.needsUpdate=!0):P.material.needsUpdate=!0)});for(let P=0,O=w.length;P<O;P++){let G=w[P],U=G.shadow;if(U===void 0){Xe("WebGLShadowMap:",G,"has no shadow.");continue}if(U.autoUpdate===!1&&U.needsUpdate===!1)continue;i.copy(U.mapSize);let ee=U.getFrameExtents();i.multiply(ee),r.copy(U.mapSize),(i.x>u||i.y>u)&&(i.x>u&&(r.x=Math.floor(u/ee.x),i.x=r.x*ee.x,U.mapSize.x=r.x),i.y>u&&(r.y=Math.floor(u/ee.y),i.y=r.y*ee.y,U.mapSize.y=r.y));let W=s.state.buffers.depth.getReversed();if(U.camera._reversedDepth=W,U.map===null||k===!0){if(U.map!==null&&(U.map.depthTexture!==null&&(U.map.depthTexture.dispose(),U.map.depthTexture=null),U.map.dispose()),this.type===Gs){if(G.isPointLight){Xe("WebGLShadowMap: VSM shadow maps are not supported for PointLights. Use PCF or BasicShadowMap instead.");continue}U.map=new hn(i.x,i.y,{format:Ci,type:In,minFilter:Dt,magFilter:Dt,generateMipmaps:!1}),U.map.texture.name=G.name+".shadowMap",U.map.depthTexture=new oi(i.x,i.y,dn),U.map.depthTexture.name=G.name+".shadowMapDepth",U.map.depthTexture.format=ii,U.map.depthTexture.compareFunction=null,U.map.depthTexture.minFilter=Vt,U.map.depthTexture.magFilter=Vt}else G.isPointLight?(U.map=new Qr(i.x),U.map.depthTexture=new br(i.x,Pn)):(U.map=new hn(i.x,i.y),U.map.depthTexture=new oi(i.x,i.y,Pn)),U.map.depthTexture.name=G.name+".shadowMap",U.map.depthTexture.format=ii,this.type===Gr?(U.map.depthTexture.compareFunction=W?Wc:Hc,U.map.depthTexture.minFilter=Dt,U.map.depthTexture.magFilter=Dt):(U.map.depthTexture.compareFunction=null,U.map.depthTexture.minFilter=Vt,U.map.depthTexture.magFilter=Vt);U.camera.updateProjectionMatrix()}U.map.isWebGLCubeRenderTarget!==!0&&(U.map.width!==i.x||U.map.height!==i.y)&&U.map.setSize(i.x,i.y);let K=U.map.isWebGLCubeRenderTarget?6:U.getViewportCount();G.isPointLight!==!0&&U.updateMatrices(G,M);let $=(U.camera.layers.mask&4294967294)!==0?U.camera.layers:M.layers;for(let he=0;he<K;he++){let de=U.getCamera(he);if(G.isPointLight){let q=U.camera,ie=U.matrix,ce=G.distance||q.far;ce!==q.far&&(q.far=ce,q.updateProjectionMatrix()),Kr.setFromMatrixPosition(G.matrixWorld),q.position.copy(Kr),oh.copy(q.position),oh.add(hy[he]),q.up.copy(uy[he]),q.lookAt(oh),q.updateMatrixWorld(),ie.makeTranslation(-Kr.x,-Kr.y,-Kr.z),nd.multiplyMatrices(q.projectionMatrix,q.matrixWorldInverse),U._frustum.setFromProjectionMatrix(nd,q.coordinateSystem,q.reversedDepth)}if(U.map.isWebGLCubeRenderTarget)s.setRenderTarget(U.map,he),s.clear();else{he===0&&(s.setRenderTarget(U.map),s.clear());let q=U.getViewport(he);o.set(r.x*q.x,r.y*q.y,r.x*q.z,r.y*q.w),D.viewport(o)}n=U.getFrustum(he),x(y,M,de,G,$,this.type)}U.isPointLightShadow!==!0&&this.type===Gs&&S(U,M),U.needsUpdate=!1}g=this.type,p.needsUpdate=!1,s.setRenderTarget(C,E,R)};function S(w,y){let M=e.update(_);d.defines.VSM_SAMPLES!==w.blurSamples&&(d.defines.VSM_SAMPLES=w.blurSamples,f.defines.VSM_SAMPLES=w.blurSamples,d.needsUpdate=!0,f.needsUpdate=!0),w.mapPass===null?w.mapPass=new hn(i.x,i.y,{format:Ci,type:In}):(w.mapPass.width!==w.map.width||w.mapPass.height!==w.map.height)&&w.mapPass.setSize(w.map.width,w.map.height),d.uniforms.shadow_pass.value=w.map.depthTexture,d.uniforms.resolution.value.set(w.map.width,w.map.height),d.uniforms.radius.value=w.radius,s.setRenderTarget(w.mapPass),s.clear(),s.renderBufferDirect(y,null,M,d,_,null),f.uniforms.shadow_pass.value=w.mapPass.texture,f.uniforms.resolution.value.set(w.map.width,w.map.height),f.uniforms.radius.value=w.radius,s.setRenderTarget(w.map),s.clear(),s.renderBufferDirect(y,null,M,f,_,null)}function v(w,y,M,C){let E=null,R=M.isPointLight===!0?w.customDistanceMaterial:w.customDepthMaterial;if(R!==void 0)E=R;else if(E=M.isPointLight===!0?l:a,s.localClippingEnabled&&y.clipShadows===!0&&Array.isArray(y.clippingPlanes)&&y.clippingPlanes.length!==0||y.displacementMap&&y.displacementScale!==0||y.alphaMap&&y.alphaTest>0||y.map&&y.alphaTest>0||y.alphaToCoverage===!0){let D=E.uuid,k=y.uuid,P=c[D];P===void 0&&(P={},c[D]=P);let O=P[k];O===void 0&&(O=E.clone(),P[k]=O,y.addEventListener("dispose",b)),E=O}if(E.visible=y.visible,E.wireframe=y.wireframe,C===Gs?E.side=y.shadowSide!==null?y.shadowSide:y.side:E.side=y.shadowSide!==null?y.shadowSide:h[y.side],E.alphaMap=y.alphaMap,E.alphaTest=y.alphaToCoverage===!0?.5:y.alphaTest,E.map=y.map,E.clipShadows=y.clipShadows,E.clippingPlanes=y.clippingPlanes,E.clipIntersection=y.clipIntersection,E.displacementMap=y.displacementMap,E.displacementScale=y.displacementScale,E.displacementBias=y.displacementBias,E.wireframeLinewidth=y.wireframeLinewidth,E.linewidth=y.linewidth,M.isPointLight===!0&&E.isMeshDistanceMaterial===!0){let D=s.properties.get(E);D.light=M}return E}function x(w,y,M,C,E,R){if(w.visible===!1)return;if(w.layers.test(E)&&(w.isMesh||w.isLine||w.isPoints)&&(w.castShadow||w.receiveShadow&&R===Gs)&&(!w.frustumCulled||w.intersectsFrustum(n))){w.modelViewMatrix.multiplyMatrices(M.matrixWorldInverse,w.matrixWorld);let P=e.update(w),O=w.material;if(Array.isArray(O)){let G=P.groups;for(let U=0,ee=G.length;U<ee;U++){let W=G[U],K=O[W.materialIndex];if(K&&K.visible){let $=v(w,K,C,R);w.onBeforeShadow(s,w,y,M,P,$,W),s.renderBufferDirect(M,null,P,$,w,W),w.onAfterShadow(s,w,y,M,P,$,W)}}}else if(O.visible){let G=v(w,O,C,R);w.onBeforeShadow(s,w,y,M,P,G,null),s.renderBufferDirect(M,null,P,G,w,null),w.onAfterShadow(s,w,y,M,P,G,null)}}let k=w.children;for(let P=0,O=k.length;P<O;P++)x(k[P],y,M,C,E,R)}function b(w){w.target.removeEventListener("dispose",b);for(let M in c){let C=c[M],E=w.target.uuid;E in C&&(C[E].dispose(),delete C[E])}}}function dy(s,e){function t(){let F=!1,oe=new _t,te=null,we=new _t(0,0,0,0);return{setMask:function(ye){te!==ye&&!F&&(s.colorMask(ye,ye,ye,ye),te=ye)},setLocked:function(ye){F=ye},setClear:function(ye,Se,Ge,tt,_e){_e===!0&&(ye*=tt,Se*=tt,Ge*=tt),oe.set(ye,Se,Ge,tt),we.equals(oe)===!1&&(s.clearColor(ye,Se,Ge,tt),we.copy(oe))},reset:function(){F=!1,te=null,we.set(-1,0,0,0)}}}function n(){let F=!1,oe=!1,te=null,we=null,ye=null;return{setReversed:function(Se){if(oe!==Se){let Ge=e.get("EXT_clip_control");Se?Ge.clipControlEXT(Ge.LOWER_LEFT_EXT,Ge.ZERO_TO_ONE_EXT):Ge.clipControlEXT(Ge.LOWER_LEFT_EXT,Ge.NEGATIVE_ONE_TO_ONE_EXT),oe=Se;let tt=ye;ye=null,this.setClear(tt)}},getReversed:function(){return oe},setTest:function(Se){Se?ue(s.DEPTH_TEST):H(s.DEPTH_TEST)},setMask:function(Se){te!==Se&&!F&&(s.depthMask(Se),te=Se)},setFunc:function(Se){if(oe&&(Se=Tf[Se]),we!==Se){switch(Se){case Xo:s.depthFunc(s.NEVER);break;case qo:s.depthFunc(s.ALWAYS);break;case Yo:s.depthFunc(s.LESS);break;case bs:s.depthFunc(s.LEQUAL);break;case Zo:s.depthFunc(s.EQUAL);break;case Jo:s.depthFunc(s.GEQUAL);break;case $o:s.depthFunc(s.GREATER);break;case Ko:s.depthFunc(s.NOTEQUAL);break;default:s.depthFunc(s.LEQUAL)}we=Se}},setLocked:function(Se){F=Se},setClear:function(Se){ye!==Se&&(ye=Se,oe&&(Se=1-Se),s.clearDepth(Se))},reset:function(){F=!1,te=null,we=null,ye=null,oe=!1}}}function i(){let F=!1,oe=null,te=null,we=null,ye=null,Se=null,Ge=null,tt=null,_e=null;return{setTest:function(We){F||(We?ue(s.STENCIL_TEST):H(s.STENCIL_TEST))},setMask:function(We){oe!==We&&!F&&(s.stencilMask(We),oe=We)},setFunc:function(We,et,Pe){(te!==We||we!==et||ye!==Pe)&&(s.stencilFunc(We,et,Pe),te=We,we=et,ye=Pe)},setOp:function(We,et,Pe){(Se!==We||Ge!==et||tt!==Pe)&&(s.stencilOp(We,et,Pe),Se=We,Ge=et,tt=Pe)},setLocked:function(We){F=We},setClear:function(We){_e!==We&&(s.clearStencil(We),_e=We)},reset:function(){F=!1,oe=null,te=null,we=null,ye=null,Se=null,Ge=null,tt=null,_e=null}}}let r=new t,o=new n,a=new i,l={},c=new WeakMap,u={},h={},d={},f=new WeakMap,m=[],_=null,p=!1,g=null,S=null,v=null,x=null,b=null,w=null,y=null,M=new Ke(0,0,0),C=0,E=!1,R=null,D=null,k=null,P=null,O=null,G=s.getParameter(s.MAX_COMBINED_TEXTURE_IMAGE_UNITS),U=!1,ee=0,W=s.getParameter(s.VERSION);W.indexOf("WebGL")!==-1?(ee=parseFloat(/^WebGL (\d)/.exec(W)[1]),U=ee>=1):W.indexOf("OpenGL ES")!==-1&&(ee=parseFloat(/^OpenGL ES (\d)/.exec(W)[1]),U=ee>=2);let K=null,$={},he=s.getParameter(s.SCISSOR_BOX),de=s.getParameter(s.VIEWPORT),q=new _t().fromArray(he),ie=new _t().fromArray(de);function ce(F,oe,te,we){let ye=new Uint8Array(4),Se=s.createTexture();s.bindTexture(F,Se),s.texParameteri(F,s.TEXTURE_MIN_FILTER,s.NEAREST),s.texParameteri(F,s.TEXTURE_MAG_FILTER,s.NEAREST);for(let Ge=0;Ge<te;Ge++)F===s.TEXTURE_3D||F===s.TEXTURE_2D_ARRAY?s.texImage3D(oe,0,s.RGBA,1,1,we,0,s.RGBA,s.UNSIGNED_BYTE,ye):s.texImage2D(oe+Ge,0,s.RGBA,1,1,0,s.RGBA,s.UNSIGNED_BYTE,ye);return Se}let Ce={};Ce[s.TEXTURE_2D]=ce(s.TEXTURE_2D,s.TEXTURE_2D,1),Ce[s.TEXTURE_CUBE_MAP]=ce(s.TEXTURE_CUBE_MAP,s.TEXTURE_CUBE_MAP_POSITIVE_X,6),Ce[s.TEXTURE_2D_ARRAY]=ce(s.TEXTURE_2D_ARRAY,s.TEXTURE_2D_ARRAY,1,1),Ce[s.TEXTURE_3D]=ce(s.TEXTURE_3D,s.TEXTURE_3D,1,1),r.setClear(0,0,0,1),o.setClear(1),a.setClear(0),ue(s.DEPTH_TEST),o.setFunc(bs),ge(!1),le(bc),ue(s.CULL_FACE),X($i);function ue(F){u[F]!==!0&&(s.enable(F),u[F]=!0)}function H(F){u[F]!==!1&&(s.disable(F),u[F]=!1)}function se(F,oe){return d[F]!==oe?(s.bindFramebuffer(F,oe),d[F]=oe,F===s.DRAW_FRAMEBUFFER&&(d[s.FRAMEBUFFER]=oe),F===s.FRAMEBUFFER&&(d[s.DRAW_FRAMEBUFFER]=oe),!0):!1}function fe(F,oe){let te=m,we=!1;if(F){te=f.get(oe),te===void 0&&(te=[],f.set(oe,te));let ye=F.textures;if(te.length!==ye.length||te[0]!==s.COLOR_ATTACHMENT0){for(let Se=0,Ge=ye.length;Se<Ge;Se++)te[Se]=s.COLOR_ATTACHMENT0+Se;te.length=ye.length,we=!0}}else te[0]!==s.BACK&&(te[0]=s.BACK,we=!0);we&&s.drawBuffers(te)}function Le(F){return _!==F?(s.useProgram(F),_=F,!0):!1}let V={[Ki]:s.FUNC_ADD,[Xu]:s.FUNC_SUBTRACT,[qu]:s.FUNC_REVERSE_SUBTRACT};V[Yu]=s.MIN,V[Zu]=s.MAX;let N={[Ju]:s.ZERO,[$u]:s.ONE,[Ku]:s.SRC_COLOR,[Ec]:s.SRC_ALPHA,[sf]:s.SRC_ALPHA_SATURATE,[tf]:s.DST_COLOR,[Qu]:s.DST_ALPHA,[ju]:s.ONE_MINUS_SRC_COLOR,[Rc]:s.ONE_MINUS_SRC_ALPHA,[nf]:s.ONE_MINUS_DST_COLOR,[ef]:s.ONE_MINUS_DST_ALPHA,[rf]:s.CONSTANT_COLOR,[of]:s.ONE_MINUS_CONSTANT_COLOR,[af]:s.CONSTANT_ALPHA,[lf]:s.ONE_MINUS_CONSTANT_ALPHA};function X(F,oe,te,we,ye,Se,Ge,tt,_e,We){if(F===$i){p===!0&&(H(s.BLEND),p=!1);return}if(p===!1&&(ue(s.BLEND),p=!0),F!==Wu){if(F!==g||We!==E){if((S!==Ki||b!==Ki)&&(s.blendEquation(s.FUNC_ADD),S=Ki,b=Ki),We)switch(F){case Hs:s.blendFuncSeparate(s.ONE,s.ONE_MINUS_SRC_ALPHA,s.ONE,s.ONE_MINUS_SRC_ALPHA);break;case wc:s.blendFunc(s.ONE,s.ONE);break;case Ac:s.blendFuncSeparate(s.ZERO,s.ONE_MINUS_SRC_COLOR,s.ZERO,s.ONE);break;case Tc:s.blendFuncSeparate(s.DST_COLOR,s.ONE_MINUS_SRC_ALPHA,s.ZERO,s.ONE);break;default:Ye("WebGLState: Invalid blending: ",F);break}else switch(F){case Hs:s.blendFuncSeparate(s.SRC_ALPHA,s.ONE_MINUS_SRC_ALPHA,s.ONE,s.ONE_MINUS_SRC_ALPHA);break;case wc:s.blendFuncSeparate(s.SRC_ALPHA,s.ONE,s.ONE,s.ONE);break;case Ac:Ye("WebGLState: SubtractiveBlending requires material.premultipliedAlpha = true");break;case Tc:Ye("WebGLState: MultiplyBlending requires material.premultipliedAlpha = true");break;default:Ye("WebGLState: Invalid blending: ",F);break}v=null,x=null,w=null,y=null,M.set(0,0,0),C=0,g=F,E=We}return}ye=ye||oe,Se=Se||te,Ge=Ge||we,(oe!==S||ye!==b)&&(s.blendEquationSeparate(V[oe],V[ye]),S=oe,b=ye),(te!==v||we!==x||Se!==w||Ge!==y)&&(s.blendFuncSeparate(N[te],N[we],N[Se],N[Ge]),v=te,x=we,w=Se,y=Ge),(tt.equals(M)===!1||_e!==C)&&(s.blendColor(tt.r,tt.g,tt.b,_e),M.copy(tt),C=_e),g=F,E=!1}function j(F,oe){F.side===Gn?H(s.CULL_FACE):ue(s.CULL_FACE);let te=F.side===Xt;oe&&(te=!te),ge(te),F.blending===Hs&&F.transparent===!1?X($i):X(F.blending,F.blendEquation,F.blendSrc,F.blendDst,F.blendEquationAlpha,F.blendSrcAlpha,F.blendDstAlpha,F.blendColor,F.blendAlpha,F.premultipliedAlpha),o.setFunc(F.depthFunc),o.setTest(F.depthTest),o.setMask(F.depthWrite),r.setMask(F.colorWrite);let we=F.stencilWrite;a.setTest(we),we&&(a.setMask(F.stencilWriteMask),a.setFunc(F.stencilFunc,F.stencilRef,F.stencilFuncMask),a.setOp(F.stencilFail,F.stencilZFail,F.stencilZPass)),xe(F.polygonOffset,F.polygonOffsetFactor,F.polygonOffsetUnits),F.alphaToCoverage===!0?ue(s.SAMPLE_ALPHA_TO_COVERAGE):H(s.SAMPLE_ALPHA_TO_COVERAGE)}function ge(F){R!==F&&(F?s.frontFace(s.CW):s.frontFace(s.CCW),R=F)}function le(F){F!==Vu?(ue(s.CULL_FACE),F!==D&&(F===bc?s.cullFace(s.BACK):F===Gu?s.cullFace(s.FRONT):s.cullFace(s.FRONT_AND_BACK))):H(s.CULL_FACE),D=F}function Re(F){F!==k&&(U&&s.lineWidth(F),k=F)}function xe(F,oe,te){F?(ue(s.POLYGON_OFFSET_FILL),(P!==oe||O!==te)&&(P=oe,O=te,o.getReversed()&&(oe=-oe,te=-te),s.polygonOffset(oe,te))):H(s.POLYGON_OFFSET_FILL)}function Ue(F){F?ue(s.SCISSOR_TEST):H(s.SCISSOR_TEST)}function Je(F){F===void 0&&(F=s.TEXTURE0+G-1),K!==F&&(s.activeTexture(F),K=F)}function Qe(F,oe,te){te===void 0&&(K===null?te=s.TEXTURE0+G-1:te=K);let we=$[te];we===void 0&&(we={type:void 0,texture:void 0},$[te]=we),(we.type!==F||we.texture!==oe)&&(K!==te&&(s.activeTexture(te),K=te),s.bindTexture(F,oe||Ce[F]),we.type=F,we.texture=oe)}function B(){let F=$[K];F!==void 0&&F.type!==void 0&&(s.bindTexture(F.type,null),F.type=void 0,F.texture=void 0)}function xt(){try{s.compressedTexImage2D(...arguments)}catch(F){Ye("WebGLState:",F)}}function je(){try{s.compressedTexImage3D(...arguments)}catch(F){Ye("WebGLState:",F)}}function rt(){try{s.texSubImage2D(...arguments)}catch(F){Ye("WebGLState:",F)}}function Me(){try{s.texSubImage3D(...arguments)}catch(F){Ye("WebGLState:",F)}}function yt(){try{s.compressedTexSubImage2D(...arguments)}catch(F){Ye("WebGLState:",F)}}function He(){try{s.compressedTexSubImage3D(...arguments)}catch(F){Ye("WebGLState:",F)}}function qe(){try{s.texStorage2D(...arguments)}catch(F){Ye("WebGLState:",F)}}function I(){try{s.texStorage3D(...arguments)}catch(F){Ye("WebGLState:",F)}}function A(){try{s.texImage2D(...arguments)}catch(F){Ye("WebGLState:",F)}}function Z(){try{s.texImage3D(...arguments)}catch(F){Ye("WebGLState:",F)}}function me(F){return h[F]!==void 0?h[F]:s.getParameter(F)}function ae(F,oe){h[F]!==oe&&(s.pixelStorei(F,oe),h[F]=oe)}function ve(F){q.equals(F)===!1&&(s.scissor(F.x,F.y,F.z,F.w),q.copy(F))}function Te(F){ie.equals(F)===!1&&(s.viewport(F.x,F.y,F.z,F.w),ie.copy(F))}function ne(F,oe){let te=c.get(oe);te===void 0&&(te=new WeakMap,c.set(oe,te));let we=te.get(F);we===void 0&&(we=s.getUniformBlockIndex(oe,F.name),s.uniformBlockBinding(oe,we,we),te.set(F,we))}function re(F,oe,te){let ye=c.get(oe).get(F);l[ye]!==te&&(s.bindBufferBase(s.UNIFORM_BUFFER,ye,te),l[ye]=te)}function be(){s.disable(s.BLEND),s.disable(s.CULL_FACE),s.disable(s.DEPTH_TEST),s.disable(s.POLYGON_OFFSET_FILL),s.disable(s.SCISSOR_TEST),s.disable(s.STENCIL_TEST),s.disable(s.SAMPLE_ALPHA_TO_COVERAGE),s.blendEquation(s.FUNC_ADD),s.blendFunc(s.ONE,s.ZERO),s.blendFuncSeparate(s.ONE,s.ZERO,s.ONE,s.ZERO),s.blendColor(0,0,0,0),s.colorMask(!0,!0,!0,!0),s.clearColor(0,0,0,0),s.depthMask(!0),s.depthFunc(s.LESS),o.setReversed(!1),s.clearDepth(1),s.stencilMask(4294967295),s.stencilFunc(s.ALWAYS,0,4294967295),s.stencilOp(s.KEEP,s.KEEP,s.KEEP),s.clearStencil(0),s.cullFace(s.BACK),s.frontFace(s.CCW),s.polygonOffset(0,0),s.activeTexture(s.TEXTURE0),s.bindFramebuffer(s.FRAMEBUFFER,null),s.bindFramebuffer(s.DRAW_FRAMEBUFFER,null),s.bindFramebuffer(s.READ_FRAMEBUFFER,null),s.useProgram(null),s.lineWidth(1),s.scissor(0,0,s.canvas.width,s.canvas.height),s.viewport(0,0,s.canvas.width,s.canvas.height),s.pixelStorei(s.PACK_ALIGNMENT,4),s.pixelStorei(s.UNPACK_ALIGNMENT,4),s.pixelStorei(s.UNPACK_FLIP_Y_WEBGL,!1),s.pixelStorei(s.UNPACK_PREMULTIPLY_ALPHA_WEBGL,!1),s.pixelStorei(s.UNPACK_COLORSPACE_CONVERSION_WEBGL,s.BROWSER_DEFAULT_WEBGL),s.pixelStorei(s.PACK_ROW_LENGTH,0),s.pixelStorei(s.PACK_SKIP_PIXELS,0),s.pixelStorei(s.PACK_SKIP_ROWS,0),s.pixelStorei(s.UNPACK_ROW_LENGTH,0),s.pixelStorei(s.UNPACK_IMAGE_HEIGHT,0),s.pixelStorei(s.UNPACK_SKIP_PIXELS,0),s.pixelStorei(s.UNPACK_SKIP_ROWS,0),s.pixelStorei(s.UNPACK_SKIP_IMAGES,0),u={},h={},K=null,$={},d={},l={},f=new WeakMap,m=[],_=null,p=!1,g=null,S=null,v=null,x=null,b=null,w=null,y=null,M=new Ke(0,0,0),C=0,E=!1,R=null,D=null,k=null,P=null,O=null,q.set(0,0,s.canvas.width,s.canvas.height),ie.set(0,0,s.canvas.width,s.canvas.height),r.reset(),o.reset(),a.reset()}return{buffers:{color:r,depth:o,stencil:a},enable:ue,disable:H,bindFramebuffer:se,drawBuffers:fe,useProgram:Le,setBlending:X,setMaterial:j,setFlipSided:ge,setCullFace:le,setLineWidth:Re,setPolygonOffset:xe,setScissorTest:Ue,activeTexture:Je,bindTexture:Qe,unbindTexture:B,compressedTexImage2D:xt,compressedTexImage3D:je,texImage2D:A,texImage3D:Z,pixelStorei:ae,getParameter:me,updateUBOMapping:ne,uniformBlockBinding:re,texStorage2D:qe,texStorage3D:I,texSubImage2D:rt,texSubImage3D:Me,compressedTexSubImage2D:yt,compressedTexSubImage3D:He,scissor:ve,viewport:Te,reset:be}}function py(s,e,t,n,i,r,o){let a=e.has("WEBGL_multisampled_render_to_texture")?e.get("WEBGL_multisampled_render_to_texture"):null,l=typeof navigator>"u"?!1:/OculusBrowser/g.test(navigator.userAgent),c=new pe,u=new WeakMap,h=new Set,d,f=new WeakMap,m=new Set,_=new FinalizationRegistry(I=>m.delete(I)),p=!1;try{p=typeof OffscreenCanvas<"u"&&new OffscreenCanvas(1,1).getContext("2d")!==null}catch{}function g(I,A){return p?new OffscreenCanvas(I,A):Ts("canvas")}function S(I,A,Z){let me=1,ae=qe(I);if((ae.width>Z||ae.height>Z)&&(me=Z/Math.max(ae.width,ae.height)),me<1)if(typeof HTMLImageElement<"u"&&I instanceof HTMLImageElement||typeof HTMLCanvasElement<"u"&&I instanceof HTMLCanvasElement||typeof ImageBitmap<"u"&&I instanceof ImageBitmap||typeof VideoFrame<"u"&&I instanceof VideoFrame){let ve=Math.floor(me*ae.width),Te=Math.floor(me*ae.height);d===void 0&&(d=g(ve,Te));let ne=A?g(ve,Te):d;return ne.width=ve,ne.height=Te,ne.getContext("2d").drawImage(I,0,0,ve,Te),Xe("WebGLRenderer: Texture has been resized from ("+ae.width+"x"+ae.height+") to ("+ve+"x"+Te+")."),ne}else return"data"in I&&Xe("WebGLRenderer: Image in DataTexture is too big ("+ae.width+"x"+ae.height+")."),I;return I}function v(I){return I.generateMipmaps}function x(I){s.generateMipmap(I)}function b(I){return I.isWebGLCubeRenderTarget?s.TEXTURE_CUBE_MAP:I.isWebGL3DRenderTarget?s.TEXTURE_3D:I.isWebGLArrayRenderTarget||I.isCompressedArrayTexture?s.TEXTURE_2D_ARRAY:s.TEXTURE_2D}function w(I,A,Z,me,ae,ve=!1){if(I!==null){if(s[I]!==void 0)return s[I];Xe("WebGLRenderer: Attempt to use non-existing WebGL internal format '"+I+"'")}let Te;me&&(Te=e.get("EXT_texture_norm16"),Te||Xe("WebGLRenderer: Unable to use normalized textures without EXT_texture_norm16 extension"));let ne=A;if(A===s.RED&&(Z===s.FLOAT&&(ne=s.R32F),Z===s.HALF_FLOAT&&(ne=s.R16F),Z===s.UNSIGNED_BYTE&&(ne=s.R8),Z===s.UNSIGNED_SHORT&&Te&&(ne=Te.R16_EXT),Z===s.SHORT&&Te&&(ne=Te.R16_SNORM_EXT)),A===s.RED_INTEGER&&(Z===s.UNSIGNED_BYTE&&(ne=s.R8UI),Z===s.UNSIGNED_SHORT&&(ne=s.R16UI),Z===s.UNSIGNED_INT&&(ne=s.R32UI),Z===s.BYTE&&(ne=s.R8I),Z===s.SHORT&&(ne=s.R16I),Z===s.INT&&(ne=s.R32I)),A===s.RG&&(Z===s.FLOAT&&(ne=s.RG32F),Z===s.HALF_FLOAT&&(ne=s.RG16F),Z===s.UNSIGNED_BYTE&&(ne=s.RG8),Z===s.UNSIGNED_SHORT&&Te&&(ne=Te.RG16_EXT),Z===s.SHORT&&Te&&(ne=Te.RG16_SNORM_EXT)),A===s.RG_INTEGER&&(Z===s.UNSIGNED_BYTE&&(ne=s.RG8UI),Z===s.UNSIGNED_SHORT&&(ne=s.RG16UI),Z===s.UNSIGNED_INT&&(ne=s.RG32UI),Z===s.BYTE&&(ne=s.RG8I),Z===s.SHORT&&(ne=s.RG16I),Z===s.INT&&(ne=s.RG32I)),A===s.RGB_INTEGER&&(Z===s.UNSIGNED_BYTE&&(ne=s.RGB8UI),Z===s.UNSIGNED_SHORT&&(ne=s.RGB16UI),Z===s.UNSIGNED_INT&&(ne=s.RGB32UI),Z===s.BYTE&&(ne=s.RGB8I),Z===s.SHORT&&(ne=s.RGB16I),Z===s.INT&&(ne=s.RGB32I)),A===s.RGBA_INTEGER&&(Z===s.UNSIGNED_BYTE&&(ne=s.RGBA8UI),Z===s.UNSIGNED_SHORT&&(ne=s.RGBA16UI),Z===s.UNSIGNED_INT&&(ne=s.RGBA32UI),Z===s.BYTE&&(ne=s.RGBA8I),Z===s.SHORT&&(ne=s.RGBA16I),Z===s.INT&&(ne=s.RGBA32I)),A===s.RGB&&(Z===s.UNSIGNED_SHORT&&Te&&(ne=Te.RGB16_EXT),Z===s.SHORT&&Te&&(ne=Te.RGB16_SNORM_EXT),Z===s.UNSIGNED_INT_5_9_9_9_REV&&(ne=s.RGB9_E5),Z===s.UNSIGNED_INT_10F_11F_11F_REV&&(ne=s.R11F_G11F_B10F)),A===s.RGBA){let re=ve?pr:lt.getTransfer(ae);Z===s.FLOAT&&(ne=s.RGBA32F),Z===s.HALF_FLOAT&&(ne=s.RGBA16F),Z===s.UNSIGNED_BYTE&&(ne=re===pt?s.SRGB8_ALPHA8:s.RGBA8),Z===s.UNSIGNED_SHORT&&Te&&(ne=Te.RGBA16_EXT),Z===s.SHORT&&Te&&(ne=Te.RGBA16_SNORM_EXT),Z===s.UNSIGNED_SHORT_4_4_4_4&&(ne=s.RGBA4),Z===s.UNSIGNED_SHORT_5_5_5_1&&(ne=s.RGB5_A1)}return(ne===s.R16F||ne===s.R32F||ne===s.RG16F||ne===s.RG32F||ne===s.RGBA16F||ne===s.RGBA32F)&&e.get("EXT_color_buffer_float"),ne}function y(I,A){let Z;return I?A===null||A===Pn||A===Qi?Z=s.DEPTH24_STENCIL8:A===dn?Z=s.DEPTH32F_STENCIL8:A===Xs&&(Z=s.DEPTH24_STENCIL8,Xe("DepthTexture: 16 bit depth attachment is not supported with stencil. Using 24-bit attachment.")):A===null||A===Pn||A===Qi?Z=s.DEPTH_COMPONENT24:A===dn?Z=s.DEPTH_COMPONENT32F:A===Xs&&(Z=s.DEPTH_COMPONENT16),Z}function M(I,A){return v(I)===!0||I.isFramebufferTexture&&I.minFilter!==Vt&&I.minFilter!==Dt?Math.log2(Math.max(A.width,A.height))+1:I.mipmaps.length>0?I.mipmaps.length:I.isCompressedTexture&&Array.isArray(I.image)?A.mipmaps.length:1}function C(I){let A=new WeakRef(I);n.get(I).__ref=A,m.add(A),_.register(I,A,A)}function E(I){let A=n.get(I).__ref;A!==void 0&&(m.delete(A),_.unregister(A))}function R(I){k(I.target)}function D(I){P(I.target)}function k(I){I.removeEventListener("dispose",R),E(I),G(I),I.isVideoTexture&&u.delete(I),I.isHTMLTexture&&h.delete(I)}function P(I){I.removeEventListener("dispose",D),E(I),ee(I)}function O(){for(let I of m){let A=I.deref();A!==void 0&&(A.isRenderTarget===!0?P(A):k(A))}m.clear(),h.clear()}function G(I){let A=n.get(I);if(A.__webglInit===void 0)return;let Z=I.source,me=f.get(Z);if(me){let ae=me[A.__cacheKey];ae.usedTimes--,ae.usedTimes===0&&U(I),Object.keys(me).length===0&&f.delete(Z)}n.remove(I)}function U(I){let A=n.get(I);s.deleteTexture(A.__webglTexture);let Z=I.source,me=f.get(Z);delete me[A.__cacheKey],o.memory.textures--}function ee(I){let A=n.get(I);A.__depthDisposeCallback&&A.__depthDisposeCallback();let Z=I.depthTexture;if(Z&&Z.renderTarget===I&&k(Z),I.isWebGLCubeRenderTarget)for(let ae=0;ae<6;ae++){if(Array.isArray(A.__webglFramebuffer[ae]))for(let ve=0;ve<A.__webglFramebuffer[ae].length;ve++)s.deleteFramebuffer(A.__webglFramebuffer[ae][ve]);else s.deleteFramebuffer(A.__webglFramebuffer[ae]);A.__webglDepthbuffer&&s.deleteRenderbuffer(A.__webglDepthbuffer[ae])}else{if(Array.isArray(A.__webglFramebuffer))for(let ae=0;ae<A.__webglFramebuffer.length;ae++)s.deleteFramebuffer(A.__webglFramebuffer[ae]);else s.deleteFramebuffer(A.__webglFramebuffer);if(A.__webglDepthbuffer&&s.deleteRenderbuffer(A.__webglDepthbuffer),A.__webglMultisampledFramebuffer&&s.deleteFramebuffer(A.__webglMultisampledFramebuffer),A.__webglColorRenderbuffer)for(let ae=0;ae<A.__webglColorRenderbuffer.length;ae++)A.__webglColorRenderbuffer[ae]&&s.deleteRenderbuffer(A.__webglColorRenderbuffer[ae]);A.__webglDepthRenderbuffer&&s.deleteRenderbuffer(A.__webglDepthRenderbuffer)}let me=I.textures;for(let ae=0,ve=me.length;ae<ve;ae++){let Te=n.get(me[ae]);Te.__webglTexture&&(s.deleteTexture(Te.__webglTexture),o.memory.textures--),n.remove(me[ae])}n.remove(I)}let W=0;function K(){W=0}function $(){return W}function he(I){W=I}function de(){let I=W;return I>=i.maxTextures&&Xe("WebGLTextures: Trying to use "+(I+1)+" texture units while this GPU supports only "+i.maxTextures),W+=1,I}function q(I){let A=[];return A.push(I.wrapS),A.push(I.wrapT),A.push(I.wrapR||0),A.push(I.magFilter),A.push(I.minFilter),A.push(I.anisotropy),A.push(I.internalFormat),A.push(I.format),A.push(I.type),A.push(I.generateMipmaps),A.push(I.mipmapsAutoUpdate),A.push(I.premultiplyAlpha),A.push(I.flipY),A.push(I.unpackAlignment),A.push(I.colorSpace),A.join()}function ie(I,A){let Z=n.get(I);if(I.isVideoTexture&&yt(I),I.isRenderTargetTexture===!1&&I.isExternalTexture!==!0&&I.version>0&&Z.__version!==I.version){let me=I.image;if(me===null)Xe("WebGLRenderer: Texture marked for update but no image data found.");else if(me.complete===!1)Xe("WebGLRenderer: Texture marked for update but image is incomplete");else{j(Z,I,A);return}}else I.isExternalTexture&&(Z.__webglTexture=I.sourceTexture?I.sourceTexture:null);t.bindTexture(s.TEXTURE_2D,Z.__webglTexture,s.TEXTURE0+A)}function ce(I,A){let Z=n.get(I);if(I.isRenderTargetTexture===!1&&I.version>0&&Z.__version!==I.version){j(Z,I,A);return}else I.isExternalTexture&&(Z.__webglTexture=I.sourceTexture?I.sourceTexture:null);t.bindTexture(s.TEXTURE_2D_ARRAY,Z.__webglTexture,s.TEXTURE0+A)}function Ce(I,A){let Z=n.get(I);if(I.isRenderTargetTexture===!1&&I.version>0&&Z.__version!==I.version){j(Z,I,A);return}t.bindTexture(s.TEXTURE_3D,Z.__webglTexture,s.TEXTURE0+A)}function ue(I,A){let Z=n.get(I);if(I.version>0&&Z.__version!==I.version){ge(Z,I,A);return}t.bindTexture(s.TEXTURE_CUBE_MAP,Z.__webglTexture,s.TEXTURE0+A)}let H={[Gi]:s.REPEAT,[_n]:s.CLAMP_TO_EDGE,[ws]:s.MIRRORED_REPEAT},se={[Vt]:s.NEAREST,[ff]:s.NEAREST_MIPMAP_NEAREST,[Hr]:s.NEAREST_MIPMAP_LINEAR,[Dt]:s.LINEAR,[ba]:s.LINEAR_MIPMAP_NEAREST,[fi]:s.LINEAR_MIPMAP_LINEAR},fe={[_f]:s.NEVER,[Xc]:s.ALWAYS,[xf]:s.LESS,[Hc]:s.LEQUAL,[yf]:s.EQUAL,[Wc]:s.GEQUAL,[vf]:s.GREATER,[Sf]:s.NOTEQUAL};function Le(I,A){if(A.type===dn&&e.has("OES_texture_float_linear")===!1&&(A.magFilter===Dt||A.magFilter===ba||A.magFilter===Hr||A.magFilter===fi||A.minFilter===Dt||A.minFilter===ba||A.minFilter===Hr||A.minFilter===fi)&&Xe("WebGLRenderer: Unable to use linear filtering with floating point textures. OES_texture_float_linear not supported on this device."),s.texParameteri(I,s.TEXTURE_WRAP_S,H[A.wrapS]),s.texParameteri(I,s.TEXTURE_WRAP_T,H[A.wrapT]),(I===s.TEXTURE_3D||I===s.TEXTURE_2D_ARRAY)&&s.texParameteri(I,s.TEXTURE_WRAP_R,H[A.wrapR]),s.texParameteri(I,s.TEXTURE_MAG_FILTER,se[A.magFilter]),s.texParameteri(I,s.TEXTURE_MIN_FILTER,se[A.minFilter]),A.compareFunction&&(s.texParameteri(I,s.TEXTURE_COMPARE_MODE,s.COMPARE_REF_TO_TEXTURE),s.texParameteri(I,s.TEXTURE_COMPARE_FUNC,fe[A.compareFunction])),e.has("EXT_texture_filter_anisotropic")===!0){if(A.magFilter===Vt||A.minFilter!==Hr&&A.minFilter!==fi||A.type===dn&&e.has("OES_texture_float_linear")===!1)return;if(A.anisotropy>1||n.get(A).__currentAnisotropy){let Z=e.get("EXT_texture_filter_anisotropic");s.texParameterf(I,Z.TEXTURE_MAX_ANISOTROPY_EXT,Math.min(A.anisotropy,i.getMaxAnisotropy())),n.get(A).__currentAnisotropy=A.anisotropy}}}function V(I,A){let Z=!1;I.__webglInit===void 0&&(I.__webglInit=!0,A.addEventListener("dispose",R),C(A));let me=A.source,ae=f.get(me);ae===void 0&&(ae={},f.set(me,ae));let ve=q(A);if(ve!==I.__cacheKey){ae[ve]===void 0&&(ae[ve]={texture:s.createTexture(),usedTimes:0},o.memory.textures++,Z=!0),ae[ve].usedTimes++;let Te=ae[I.__cacheKey];Te!==void 0&&(ae[I.__cacheKey].usedTimes--,Te.usedTimes===0&&U(A)),I.__cacheKey=ve,I.__webglTexture=ae[ve].texture}return Z}function N(I,A,Z){return Math.floor(Math.floor(I/Z)/A)}function X(I,A,Z,me){let ve=I.updateRanges;if(ve.length===0)t.texSubImage2D(s.TEXTURE_2D,0,0,0,A.width,A.height,Z,me,A.data);else{ve.sort((F,oe)=>F.start-oe.start);let Te=0;for(let F=1;F<ve.length;F++){let oe=ve[Te],te=ve[F],we=oe.start+oe.count,ye=N(te.start,A.width,4),Se=N(oe.start,A.width,4);te.start<=we+1&&ye===Se&&N(te.start+te.count-1,A.width,4)===ye?oe.count=Math.max(oe.count,te.start+te.count-oe.start):(++Te,ve[Te]=te)}ve.length=Te+1;let ne=t.getParameter(s.UNPACK_ROW_LENGTH),re=t.getParameter(s.UNPACK_SKIP_PIXELS),be=t.getParameter(s.UNPACK_SKIP_ROWS);t.pixelStorei(s.UNPACK_ROW_LENGTH,A.width);for(let F=0,oe=ve.length;F<oe;F++){let te=ve[F],we=Math.floor(te.start/4),ye=Math.ceil(te.count/4),Se=we%A.width,Ge=Math.floor(we/A.width),tt=ye,_e=1;t.pixelStorei(s.UNPACK_SKIP_PIXELS,Se),t.pixelStorei(s.UNPACK_SKIP_ROWS,Ge),t.texSubImage2D(s.TEXTURE_2D,0,Se,Ge,tt,_e,Z,me,A.data)}I.clearUpdateRanges(),t.pixelStorei(s.UNPACK_ROW_LENGTH,ne),t.pixelStorei(s.UNPACK_SKIP_PIXELS,re),t.pixelStorei(s.UNPACK_SKIP_ROWS,be)}}function j(I,A,Z){let me=s.TEXTURE_2D;(A.isDataArrayTexture||A.isCompressedArrayTexture)&&(me=s.TEXTURE_2D_ARRAY),A.isData3DTexture&&(me=s.TEXTURE_3D);let ae=V(I,A),ve=A.source;t.bindTexture(me,I.__webglTexture,s.TEXTURE0+Z);let Te=n.get(ve);if(ve.version!==Te.__version||ae===!0){if(t.activeTexture(s.TEXTURE0+Z),(typeof ImageBitmap<"u"&&A.image instanceof ImageBitmap)===!1){let _e=lt.getPrimaries(lt.workingColorSpace),We=A.colorSpace===qt?null:lt.getPrimaries(A.colorSpace),et=A.colorSpace===qt||_e===We?s.NONE:s.BROWSER_DEFAULT_WEBGL;t.pixelStorei(s.UNPACK_FLIP_Y_WEBGL,A.flipY),t.pixelStorei(s.UNPACK_PREMULTIPLY_ALPHA_WEBGL,A.premultiplyAlpha),t.pixelStorei(s.UNPACK_COLORSPACE_CONVERSION_WEBGL,et)}t.pixelStorei(s.UNPACK_ALIGNMENT,A.unpackAlignment);let re=S(A.image,!1,i.maxTextureSize);re=He(A,re);let be=r.convert(A.format,A.colorSpace),F=r.convert(A.type),oe=w(A.internalFormat,be,F,A.normalized,A.colorSpace,A.isVideoTexture);Le(me,A);let te,we=A.mipmaps,ye=A.isVideoTexture!==!0,Se=Te.__version===void 0||ae===!0,Ge=ve.dataReady,tt=M(A,re);if(A.isDepthTexture)oe=y(A.format===di,A.type),Se&&(ye?t.texStorage2D(s.TEXTURE_2D,1,oe,re.width,re.height):t.texImage2D(s.TEXTURE_2D,0,oe,re.width,re.height,0,be,F,null));else if(A.isDataTexture)if(we.length>0){ye&&Se&&t.texStorage2D(s.TEXTURE_2D,tt,oe,we[0].width,we[0].height);for(let _e=0,We=we.length;_e<We;_e++)te=we[_e],ye?Ge&&t.texSubImage2D(s.TEXTURE_2D,_e,0,0,te.width,te.height,be,F,te.data):t.texImage2D(s.TEXTURE_2D,_e,oe,te.width,te.height,0,be,F,te.data);A.generateMipmaps=!1}else ye?(Se&&t.texStorage2D(s.TEXTURE_2D,tt,oe,re.width,re.height),Ge&&X(A,re,be,F)):t.texImage2D(s.TEXTURE_2D,0,oe,re.width,re.height,0,be,F,re.data);else if(A.isCompressedTexture)if(A.isCompressedArrayTexture){ye&&Se&&t.texStorage3D(s.TEXTURE_2D_ARRAY,tt,oe,we[0].width,we[0].height,re.depth);for(let _e=0,We=we.length;_e<We;_e++)if(te=we[_e],A.format!==Mn)if(be!==null)if(ye){if(Ge)if(A.layerUpdates.size>0){let et=Qc(te.width,te.height,A.format,A.type);for(let Pe of A.layerUpdates){let mt=te.data.subarray(Pe*et/te.data.BYTES_PER_ELEMENT,(Pe+1)*et/te.data.BYTES_PER_ELEMENT);t.compressedTexSubImage3D(s.TEXTURE_2D_ARRAY,_e,0,0,Pe,te.width,te.height,1,be,mt)}}else t.compressedTexSubImage3D(s.TEXTURE_2D_ARRAY,_e,0,0,0,te.width,te.height,re.depth,be,te.data)}else t.compressedTexImage3D(s.TEXTURE_2D_ARRAY,_e,oe,te.width,te.height,re.depth,0,te.data,0,0);else Xe("WebGLRenderer: Attempt to load unsupported compressed texture format in .uploadTexture()");else ye?Ge&&t.texSubImage3D(s.TEXTURE_2D_ARRAY,_e,0,0,0,te.width,te.height,re.depth,be,F,te.data):t.texImage3D(s.TEXTURE_2D_ARRAY,_e,oe,te.width,te.height,re.depth,0,be,F,te.data);A.layerUpdates.size>0&&A.clearLayerUpdates()}else{ye&&Se&&t.texStorage2D(s.TEXTURE_2D,tt,oe,we[0].width,we[0].height);for(let _e=0,We=we.length;_e<We;_e++)te=we[_e],A.format!==Mn?be!==null?ye?Ge&&t.compressedTexSubImage2D(s.TEXTURE_2D,_e,0,0,te.width,te.height,be,te.data):t.compressedTexImage2D(s.TEXTURE_2D,_e,oe,te.width,te.height,0,te.data):Xe("WebGLRenderer: Attempt to load unsupported compressed texture format in .uploadTexture()"):ye?Ge&&t.texSubImage2D(s.TEXTURE_2D,_e,0,0,te.width,te.height,be,F,te.data):t.texImage2D(s.TEXTURE_2D,_e,oe,te.width,te.height,0,be,F,te.data)}else if(A.isDataArrayTexture)if(ye){if(Se&&t.texStorage3D(s.TEXTURE_2D_ARRAY,tt,oe,re.width,re.height,re.depth),Ge)if(A.layerUpdates.size>0){let _e=Qc(re.width,re.height,A.format,A.type);for(let We of A.layerUpdates){let et=re.data.subarray(We*_e/re.data.BYTES_PER_ELEMENT,(We+1)*_e/re.data.BYTES_PER_ELEMENT);t.texSubImage3D(s.TEXTURE_2D_ARRAY,0,0,0,We,re.width,re.height,1,be,F,et)}A.clearLayerUpdates()}else t.texSubImage3D(s.TEXTURE_2D_ARRAY,0,0,0,0,re.width,re.height,re.depth,be,F,re.data)}else t.texImage3D(s.TEXTURE_2D_ARRAY,0,oe,re.width,re.height,re.depth,0,be,F,re.data);else if(A.isData3DTexture)ye?(Se&&t.texStorage3D(s.TEXTURE_3D,tt,oe,re.width,re.height,re.depth),Ge&&t.texSubImage3D(s.TEXTURE_3D,0,0,0,0,re.width,re.height,re.depth,be,F,re.data)):t.texImage3D(s.TEXTURE_3D,0,oe,re.width,re.height,re.depth,0,be,F,re.data);else if(A.isFramebufferTexture){if(Se)if(ye)t.texStorage2D(s.TEXTURE_2D,tt,oe,re.width,re.height);else{let _e=re.width,We=re.height;for(let et=0;et<tt;et++)t.texImage2D(s.TEXTURE_2D,et,oe,_e,We,0,be,F,null),_e>>=1,We>>=1}}else if(A.isHTMLTexture){if("texElementImage2D"in s){let _e=s.canvas;if(_e.hasAttribute("layoutsubtree")||_e.setAttribute("layoutsubtree","true"),re.parentNode!==_e){_e.appendChild(re),h.add(A),_e.onpaint=We=>{let et=We.changedElements;for(let Pe of h)et.includes(Pe.image)&&(Pe.needsUpdate=!0)},_e.requestPaint();return}if(s.texElementImage2D.length===3)s.texElementImage2D(s.TEXTURE_2D,s.RGBA8,re);else{let et=s.RGBA,Pe=s.RGBA,mt=s.UNSIGNED_BYTE;s.texElementImage2D(s.TEXTURE_2D,0,et,Pe,mt,re)}s.texParameteri(s.TEXTURE_2D,s.TEXTURE_MIN_FILTER,s.LINEAR),s.texParameteri(s.TEXTURE_2D,s.TEXTURE_WRAP_S,s.CLAMP_TO_EDGE),s.texParameteri(s.TEXTURE_2D,s.TEXTURE_WRAP_T,s.CLAMP_TO_EDGE)}}else if(we.length>0){if(ye&&Se){let _e=qe(we[0]);t.texStorage2D(s.TEXTURE_2D,tt,oe,_e.width,_e.height)}for(let _e=0,We=we.length;_e<We;_e++)te=we[_e],ye?Ge&&t.texSubImage2D(s.TEXTURE_2D,_e,0,0,be,F,te):t.texImage2D(s.TEXTURE_2D,_e,oe,be,F,te);A.generateMipmaps=!1}else if(ye){if(Se){let _e=qe(re);t.texStorage2D(s.TEXTURE_2D,tt,oe,_e.width,_e.height)}Ge&&t.texSubImage2D(s.TEXTURE_2D,0,0,0,be,F,re)}else t.texImage2D(s.TEXTURE_2D,0,oe,be,F,re);v(A)&&A.mipmapsAutoUpdate===!0&&x(me),Te.__version=ve.version,A.onUpdate&&A.onUpdate(A)}I.__version=A.version}function ge(I,A,Z){if(A.image.length!==6)return;let me=V(I,A),ae=A.source;t.bindTexture(s.TEXTURE_CUBE_MAP,I.__webglTexture,s.TEXTURE0+Z);let ve=n.get(ae);if(ae.version!==ve.__version||me===!0){t.activeTexture(s.TEXTURE0+Z);let Te=lt.getPrimaries(lt.workingColorSpace),ne=A.colorSpace===qt?null:lt.getPrimaries(A.colorSpace),re=A.colorSpace===qt||Te===ne?s.NONE:s.BROWSER_DEFAULT_WEBGL;t.pixelStorei(s.UNPACK_FLIP_Y_WEBGL,A.flipY),t.pixelStorei(s.UNPACK_PREMULTIPLY_ALPHA_WEBGL,A.premultiplyAlpha),t.pixelStorei(s.UNPACK_ALIGNMENT,A.unpackAlignment),t.pixelStorei(s.UNPACK_COLORSPACE_CONVERSION_WEBGL,re);let be=A.isCompressedTexture||A.image[0].isCompressedTexture,F=A.image[0]&&A.image[0].isDataTexture,oe=[];for(let Pe=0;Pe<6;Pe++)!be&&!F?oe[Pe]=S(A.image[Pe],!0,i.maxCubemapSize):oe[Pe]=F?A.image[Pe].image:A.image[Pe],oe[Pe]=He(A,oe[Pe]);let te=oe[0],we=r.convert(A.format,A.colorSpace),ye=r.convert(A.type),Se=w(A.internalFormat,we,ye,A.normalized,A.colorSpace),Ge=A.isVideoTexture!==!0,tt=ve.__version===void 0||me===!0,_e=ae.dataReady,We=M(A,te);Le(s.TEXTURE_CUBE_MAP,A);let et;if(A.isDepthTexture){if(Se=y(A.format===di,A.type),tt)if(Ge)t.texStorage2D(s.TEXTURE_CUBE_MAP,1,Se,te.width,te.height);else for(let Pe=0;Pe<6;Pe++)t.texImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,0,Se,te.width,te.height,0,we,ye,null)}else if(be){Ge&&tt&&t.texStorage2D(s.TEXTURE_CUBE_MAP,We,Se,te.width,te.height);for(let Pe=0;Pe<6;Pe++){et=oe[Pe].mipmaps;for(let mt=0;mt<et.length;mt++){let Bt=et[mt];A.format!==Mn?we!==null?Ge?_e&&t.compressedTexSubImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,mt,0,0,Bt.width,Bt.height,we,Bt.data):t.compressedTexImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,mt,Se,Bt.width,Bt.height,0,Bt.data):Xe("WebGLRenderer: Attempt to load unsupported compressed texture format in .setTextureCube()"):Ge?_e&&t.texSubImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,mt,0,0,Bt.width,Bt.height,we,ye,Bt.data):t.texImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,mt,Se,Bt.width,Bt.height,0,we,ye,Bt.data)}}}else{if(et=A.mipmaps,Ge&&tt){et.length>0&&We++;let Pe=qe(oe[0]);t.texStorage2D(s.TEXTURE_CUBE_MAP,We,Se,Pe.width,Pe.height)}for(let Pe=0;Pe<6;Pe++)if(F){Ge?_e&&t.texSubImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,0,0,0,oe[Pe].width,oe[Pe].height,we,ye,oe[Pe].data):t.texImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,0,Se,oe[Pe].width,oe[Pe].height,0,we,ye,oe[Pe].data);for(let mt=0;mt<et.length;mt++){let gi=et[mt].image[Pe].image;Ge?_e&&t.texSubImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,mt+1,0,0,gi.width,gi.height,we,ye,gi.data):t.texImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,mt+1,Se,gi.width,gi.height,0,we,ye,gi.data)}}else{Ge?_e&&t.texSubImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,0,0,0,we,ye,oe[Pe]):t.texImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,0,Se,we,ye,oe[Pe]);for(let mt=0;mt<et.length;mt++){let Bt=et[mt];Ge?_e&&t.texSubImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,mt+1,0,0,we,ye,Bt.image[Pe]):t.texImage2D(s.TEXTURE_CUBE_MAP_POSITIVE_X+Pe,mt+1,Se,we,ye,Bt.image[Pe])}}}v(A)&&A.mipmapsAutoUpdate===!0&&x(s.TEXTURE_CUBE_MAP),ve.__version=ae.version,A.onUpdate&&A.onUpdate(A)}I.__version=A.version}function le(I,A,Z,me,ae,ve){let Te=r.convert(Z.format,Z.colorSpace),ne=r.convert(Z.type),re=w(Z.internalFormat,Te,ne,Z.normalized,Z.colorSpace),be=n.get(A),F=n.get(Z);if(F.__renderTarget=A,!be.__hasExternalTextures){let oe=Math.max(1,A.width>>ve),te=Math.max(1,A.height>>ve);ae===s.TEXTURE_3D||ae===s.TEXTURE_2D_ARRAY?t.texImage3D(ae,ve,re,oe,te,A.depth,0,Te,ne,null):t.texImage2D(ae,ve,re,oe,te,0,Te,ne,null)}t.bindFramebuffer(s.FRAMEBUFFER,I),Me(A)?a.framebufferTexture2DMultisampleEXT(s.FRAMEBUFFER,me,ae,F.__webglTexture,0,rt(A)):(ae===s.TEXTURE_2D||ae>=s.TEXTURE_CUBE_MAP_POSITIVE_X&&ae<=s.TEXTURE_CUBE_MAP_NEGATIVE_Z)&&s.framebufferTexture2D(s.FRAMEBUFFER,me,ae,F.__webglTexture,ve),t.bindFramebuffer(s.FRAMEBUFFER,null)}function Re(I,A,Z){if(s.bindRenderbuffer(s.RENDERBUFFER,I),A.depthBuffer){let me=A.depthTexture,ae=me&&me.isDepthTexture?me.type:null,ve=y(A.stencilBuffer,ae),Te=A.stencilBuffer?s.DEPTH_STENCIL_ATTACHMENT:s.DEPTH_ATTACHMENT;Me(A)?a.renderbufferStorageMultisampleEXT(s.RENDERBUFFER,rt(A),ve,A.width,A.height):Z?s.renderbufferStorageMultisample(s.RENDERBUFFER,rt(A),ve,A.width,A.height):s.renderbufferStorage(s.RENDERBUFFER,ve,A.width,A.height),s.framebufferRenderbuffer(s.FRAMEBUFFER,Te,s.RENDERBUFFER,I)}else{let me=A.textures;for(let ae=0;ae<me.length;ae++){let ve=me[ae],Te=r.convert(ve.format,ve.colorSpace),ne=r.convert(ve.type),re=w(ve.internalFormat,Te,ne,ve.normalized,ve.colorSpace);Me(A)?a.renderbufferStorageMultisampleEXT(s.RENDERBUFFER,rt(A),re,A.width,A.height):Z?s.renderbufferStorageMultisample(s.RENDERBUFFER,rt(A),re,A.width,A.height):s.renderbufferStorage(s.RENDERBUFFER,re,A.width,A.height)}}s.bindRenderbuffer(s.RENDERBUFFER,null)}function xe(I,A,Z){let me=A.isWebGLCubeRenderTarget===!0;if(t.bindFramebuffer(s.FRAMEBUFFER,I),!(A.depthTexture&&A.depthTexture.isDepthTexture))throw new Error("THREE.WebGLTextures: renderTarget.depthTexture must be an instance of THREE.DepthTexture.");let ae=n.get(A.depthTexture);ae.__renderTarget=A;let ve=me?A.depthTexture.image[0]:A.depthTexture.image;(!ae.__webglTexture||ve.width!==A.width||ve.height!==A.height)&&(ve.width=A.width,ve.height=A.height,A.depthTexture.needsUpdate=!0),me?ue(A.depthTexture,0):ie(A.depthTexture,0);let Te=ae.__webglTexture,ne=rt(A),re=me?s.TEXTURE_CUBE_MAP_POSITIVE_X+Z:s.TEXTURE_2D,be=A.depthTexture.format===di?s.DEPTH_STENCIL_ATTACHMENT:s.DEPTH_ATTACHMENT;if(A.depthTexture.format===ii)Me(A)?a.framebufferTexture2DMultisampleEXT(s.FRAMEBUFFER,be,re,Te,0,ne):s.framebufferTexture2D(s.FRAMEBUFFER,be,re,Te,0);else if(A.depthTexture.format===di)Me(A)?a.framebufferTexture2DMultisampleEXT(s.FRAMEBUFFER,be,re,Te,0,ne):s.framebufferTexture2D(s.FRAMEBUFFER,be,re,Te,0);else throw new Error("THREE.WebGLTextures: Unknown depthTexture format.")}function Ue(I){let A=n.get(I),Z=I.isWebGLCubeRenderTarget===!0;if(A.__boundDepthTexture!==I.depthTexture){let me=I.depthTexture;if(A.__depthDisposeCallback&&A.__depthDisposeCallback(),me){let ae=()=>{delete A.__boundDepthTexture,delete A.__depthDisposeCallback,me.removeEventListener("dispose",ae)};me.addEventListener("dispose",ae),A.__depthDisposeCallback=ae}A.__boundDepthTexture=me}if(I.depthTexture&&!A.__autoAllocateDepthBuffer)if(Z)for(let me=0;me<6;me++)xe(A.__webglFramebuffer[me],I,me);else I.texture.mipmaps.length>0?xe(A.__webglFramebuffer[0],I,0):xe(A.__webglFramebuffer,I,0);else if(Z){A.__webglDepthbuffer=[];for(let me=0;me<6;me++)if(t.bindFramebuffer(s.FRAMEBUFFER,A.__webglFramebuffer[me]),A.__webglDepthbuffer[me]===void 0)A.__webglDepthbuffer[me]=s.createRenderbuffer(),Re(A.__webglDepthbuffer[me],I,!1);else{let ae=I.stencilBuffer?s.DEPTH_STENCIL_ATTACHMENT:s.DEPTH_ATTACHMENT,ve=A.__webglDepthbuffer[me];s.bindRenderbuffer(s.RENDERBUFFER,ve),s.framebufferRenderbuffer(s.FRAMEBUFFER,ae,s.RENDERBUFFER,ve)}}else if(I.texture.mipmaps.length>0?t.bindFramebuffer(s.FRAMEBUFFER,A.__webglFramebuffer[0]):t.bindFramebuffer(s.FRAMEBUFFER,A.__webglFramebuffer),A.__webglDepthbuffer===void 0)A.__webglDepthbuffer=s.createRenderbuffer(),Re(A.__webglDepthbuffer,I,!1);else{let ae=I.stencilBuffer?s.DEPTH_STENCIL_ATTACHMENT:s.DEPTH_ATTACHMENT,ve=A.__webglDepthbuffer;s.bindRenderbuffer(s.RENDERBUFFER,ve),s.framebufferRenderbuffer(s.FRAMEBUFFER,ae,s.RENDERBUFFER,ve)}t.bindFramebuffer(s.FRAMEBUFFER,null)}function Je(I,A,Z){let me=n.get(I);A!==void 0&&le(me.__webglFramebuffer,I,I.texture,s.COLOR_ATTACHMENT0,s.TEXTURE_2D,0),Z!==void 0&&Ue(I)}function Qe(I){let A=I.texture,Z=n.get(I),me=n.get(A);I.addEventListener("dispose",D),C(I);let ae=I.textures,ve=I.isWebGLCubeRenderTarget===!0,Te=ae.length>1;if(Te||(me.__webglTexture===void 0&&(me.__webglTexture=s.createTexture()),me.__version=A.version,o.memory.textures++),ve){Z.__webglFramebuffer=[];for(let ne=0;ne<6;ne++)if(A.mipmaps.length>0){Z.__webglFramebuffer[ne]=[];for(let re=0;re<A.mipmaps.length;re++)Z.__webglFramebuffer[ne][re]=s.createFramebuffer()}else Z.__webglFramebuffer[ne]=s.createFramebuffer()}else{if(A.mipmaps.length>0){Z.__webglFramebuffer=[];for(let ne=0;ne<A.mipmaps.length;ne++)Z.__webglFramebuffer[ne]=s.createFramebuffer()}else Z.__webglFramebuffer=s.createFramebuffer();if(Te)for(let ne=0,re=ae.length;ne<re;ne++){let be=n.get(ae[ne]);be.__webglTexture===void 0&&(be.__webglTexture=s.createTexture(),o.memory.textures++)}if(I.samples>0&&Me(I)===!1){Z.__webglMultisampledFramebuffer=s.createFramebuffer(),Z.__webglColorRenderbuffer=[],t.bindFramebuffer(s.FRAMEBUFFER,Z.__webglMultisampledFramebuffer);for(let ne=0;ne<ae.length;ne++){let re=ae[ne];Z.__webglColorRenderbuffer[ne]=s.createRenderbuffer(),s.bindRenderbuffer(s.RENDERBUFFER,Z.__webglColorRenderbuffer[ne]);let be=r.convert(re.format,re.colorSpace),F=r.convert(re.type),oe=w(re.internalFormat,be,F,re.normalized,re.colorSpace,I.isXRRenderTarget===!0),te=rt(I);s.renderbufferStorageMultisample(s.RENDERBUFFER,te,oe,I.width,I.height),s.framebufferRenderbuffer(s.FRAMEBUFFER,s.COLOR_ATTACHMENT0+ne,s.RENDERBUFFER,Z.__webglColorRenderbuffer[ne])}s.bindRenderbuffer(s.RENDERBUFFER,null),I.depthBuffer&&(Z.__webglDepthRenderbuffer=s.createRenderbuffer(),Re(Z.__webglDepthRenderbuffer,I,!0)),t.bindFramebuffer(s.FRAMEBUFFER,null)}}if(ve){t.bindTexture(s.TEXTURE_CUBE_MAP,me.__webglTexture),Le(s.TEXTURE_CUBE_MAP,A);for(let ne=0;ne<6;ne++)if(A.mipmaps.length>0)for(let re=0;re<A.mipmaps.length;re++)le(Z.__webglFramebuffer[ne][re],I,A,s.COLOR_ATTACHMENT0,s.TEXTURE_CUBE_MAP_POSITIVE_X+ne,re);else le(Z.__webglFramebuffer[ne],I,A,s.COLOR_ATTACHMENT0,s.TEXTURE_CUBE_MAP_POSITIVE_X+ne,0);v(A)?x(s.TEXTURE_CUBE_MAP):A.mipmaps.length>0&&s.texParameteri(s.TEXTURE_CUBE_MAP,s.TEXTURE_MAX_LEVEL,A.mipmaps.length-1),t.unbindTexture()}else if(Te){for(let ne=0,re=ae.length;ne<re;ne++){let be=ae[ne],F=n.get(be),oe=s.TEXTURE_2D;(I.isWebGL3DRenderTarget||I.isWebGLArrayRenderTarget)&&(oe=I.isWebGL3DRenderTarget?s.TEXTURE_3D:s.TEXTURE_2D_ARRAY),t.bindTexture(oe,F.__webglTexture),Le(oe,be),le(Z.__webglFramebuffer,I,be,s.COLOR_ATTACHMENT0+ne,oe,0),v(be)&&x(oe)}t.unbindTexture()}else{let ne=s.TEXTURE_2D;if((I.isWebGL3DRenderTarget||I.isWebGLArrayRenderTarget)&&(ne=I.isWebGL3DRenderTarget?s.TEXTURE_3D:s.TEXTURE_2D_ARRAY),t.bindTexture(ne,me.__webglTexture),Le(ne,A),A.mipmaps.length>0)for(let re=0;re<A.mipmaps.length;re++)le(Z.__webglFramebuffer[re],I,A,s.COLOR_ATTACHMENT0,ne,re);else le(Z.__webglFramebuffer,I,A,s.COLOR_ATTACHMENT0,ne,0);v(A)?x(ne):A.mipmaps.length>0&&s.texParameteri(ne,s.TEXTURE_MAX_LEVEL,A.mipmaps.length-1),t.unbindTexture()}I.depthBuffer&&Ue(I)}function B(I){let A=I.textures;for(let Z=0,me=A.length;Z<me;Z++){let ae=A[Z];if(v(ae)&&ae.mipmapsAutoUpdate===!0){let ve=b(I),Te=n.get(ae).__webglTexture;t.bindTexture(ve,Te),x(ve),t.unbindTexture()}}}let xt=[];function je(I){if(I.samples>0){if(Me(I)===!1){let A=I.textures,Z=I.width,me=I.height,ae=s.COLOR_BUFFER_BIT,ve=I.stencilBuffer?s.DEPTH_STENCIL_ATTACHMENT:s.DEPTH_ATTACHMENT,Te=n.get(I),ne=A.length>1;if(ne)for(let be=0;be<A.length;be++)t.bindFramebuffer(s.FRAMEBUFFER,Te.__webglMultisampledFramebuffer),s.framebufferRenderbuffer(s.FRAMEBUFFER,s.COLOR_ATTACHMENT0+be,s.RENDERBUFFER,null),t.bindFramebuffer(s.FRAMEBUFFER,Te.__webglFramebuffer),s.framebufferTexture2D(s.DRAW_FRAMEBUFFER,s.COLOR_ATTACHMENT0+be,s.TEXTURE_2D,null,0);t.bindFramebuffer(s.READ_FRAMEBUFFER,Te.__webglMultisampledFramebuffer),I.texture.mipmaps.length>0?t.bindFramebuffer(s.DRAW_FRAMEBUFFER,Te.__webglFramebuffer[0]):t.bindFramebuffer(s.DRAW_FRAMEBUFFER,Te.__webglFramebuffer);for(let be=0;be<A.length;be++){if(I.resolveDepthBuffer&&(I.depthBuffer&&(ae|=s.DEPTH_BUFFER_BIT),I.stencilBuffer&&I.resolveStencilBuffer&&(ae|=s.STENCIL_BUFFER_BIT)),ne){s.framebufferRenderbuffer(s.READ_FRAMEBUFFER,s.COLOR_ATTACHMENT0,s.RENDERBUFFER,Te.__webglColorRenderbuffer[be]);let F=n.get(A[be]).__webglTexture;s.framebufferTexture2D(s.DRAW_FRAMEBUFFER,s.COLOR_ATTACHMENT0,s.TEXTURE_2D,F,0)}s.blitFramebuffer(0,0,Z,me,0,0,Z,me,ae,s.NEAREST),l===!0&&(xt.length=0,xt.push(s.COLOR_ATTACHMENT0+be),I.depthBuffer&&I.storeMultisampledDepthBuffer===!1&&xt.push(ve),s.invalidateFramebuffer(s.READ_FRAMEBUFFER,xt))}if(t.bindFramebuffer(s.READ_FRAMEBUFFER,null),t.bindFramebuffer(s.DRAW_FRAMEBUFFER,null),ne)for(let be=0;be<A.length;be++){t.bindFramebuffer(s.FRAMEBUFFER,Te.__webglMultisampledFramebuffer),s.framebufferRenderbuffer(s.FRAMEBUFFER,s.COLOR_ATTACHMENT0+be,s.RENDERBUFFER,Te.__webglColorRenderbuffer[be]);let F=n.get(A[be]).__webglTexture;t.bindFramebuffer(s.FRAMEBUFFER,Te.__webglFramebuffer),s.framebufferTexture2D(s.DRAW_FRAMEBUFFER,s.COLOR_ATTACHMENT0+be,s.TEXTURE_2D,F,0)}t.bindFramebuffer(s.DRAW_FRAMEBUFFER,Te.__webglMultisampledFramebuffer)}else if(I.depthBuffer&&I.storeMultisampledDepthBuffer===!1&&l){let A=I.stencilBuffer?s.DEPTH_STENCIL_ATTACHMENT:s.DEPTH_ATTACHMENT;s.invalidateFramebuffer(s.DRAW_FRAMEBUFFER,[A])}}}function rt(I){return Math.min(i.maxSamples,I.samples)}function Me(I){let A=n.get(I);return I.samples>0&&e.has("WEBGL_multisampled_render_to_texture")===!0&&A.__useRenderToTexture!==!1}function yt(I){let A=o.render.frame;u.get(I)!==A&&(u.set(I,A),I.update())}function He(I,A){let Z=I.colorSpace,me=I.format,ae=I.type;return I.isCompressedTexture===!0||I.isVideoTexture===!0||Z!==Gt&&Z!==qt&&(lt.getTransfer(Z)===pt?(me!==Mn||ae!==on)&&Xe("WebGLTextures: sRGB encoded textures have to use RGBAFormat and UnsignedByteType."):Ye("WebGLTextures: Unsupported texture color space:",Z)),A}function qe(I){return typeof HTMLImageElement<"u"&&I instanceof HTMLImageElement?(c.width=I.naturalWidth||I.width,c.height=I.naturalHeight||I.height):typeof VideoFrame<"u"&&I instanceof VideoFrame?(c.width=I.displayWidth,c.height=I.displayHeight):(c.width=I.width,c.height=I.height),c}this.allocateTextureUnit=de,this.resetTextureUnits=K,this.getTextureUnits=$,this.setTextureUnits=he,this.setTexture2D=ie,this.setTexture2DArray=ce,this.setTexture3D=Ce,this.setTextureCube=ue,this.rebindTextures=Je,this.setupRenderTarget=Qe,this.updateRenderTargetMipmap=B,this.updateMultisampleRenderTarget=je,this.setupDepthRenderbuffer=Ue,this.setupFrameBufferTexture=le,this.useMultisampledRTT=Me,this.dispose=O}function my(s,e){function t(n,i=qt){let r,o=lt.getTransfer(i);if(n===on)return s.UNSIGNED_BYTE;if(n===Aa)return s.UNSIGNED_SHORT_4_4_4_4;if(n===Ta)return s.UNSIGNED_SHORT_5_5_5_1;if(n===zc)return s.UNSIGNED_INT_5_9_9_9_REV;if(n===kc)return s.UNSIGNED_INT_10F_11F_11F_REV;if(n===Oc)return s.BYTE;if(n===Bc)return s.SHORT;if(n===Xs)return s.UNSIGNED_SHORT;if(n===wa)return s.INT;if(n===Pn)return s.UNSIGNED_INT;if(n===dn)return s.FLOAT;if(n===In)return s.HALF_FLOAT;if(n===Vc)return s.ALPHA;if(n===Gc)return s.RGB;if(n===Mn)return s.RGBA;if(n===ii)return s.DEPTH_COMPONENT;if(n===di)return s.DEPTH_STENCIL;if(n===Ea)return s.RED;if(n===Ra)return s.RED_INTEGER;if(n===Ci)return s.RG;if(n===Ca)return s.RG_INTEGER;if(n===Pa)return s.RGBA_INTEGER;if(n===Wr||n===Xr||n===qr||n===Yr)if(o===pt)if(r=e.get("WEBGL_compressed_texture_s3tc_srgb"),r!==null){if(n===Wr)return r.COMPRESSED_SRGB_S3TC_DXT1_EXT;if(n===Xr)return r.COMPRESSED_SRGB_ALPHA_S3TC_DXT1_EXT;if(n===qr)return r.COMPRESSED_SRGB_ALPHA_S3TC_DXT3_EXT;if(n===Yr)return r.COMPRESSED_SRGB_ALPHA_S3TC_DXT5_EXT}else return null;else if(r=e.get("WEBGL_compressed_texture_s3tc"),r!==null){if(n===Wr)return r.COMPRESSED_RGB_S3TC_DXT1_EXT;if(n===Xr)return r.COMPRESSED_RGBA_S3TC_DXT1_EXT;if(n===qr)return r.COMPRESSED_RGBA_S3TC_DXT3_EXT;if(n===Yr)return r.COMPRESSED_RGBA_S3TC_DXT5_EXT}else return null;if(n===Ia||n===Da||n===La||n===Na)if(r=e.get("WEBGL_compressed_texture_pvrtc"),r!==null){if(n===Ia)return r.COMPRESSED_RGB_PVRTC_4BPPV1_IMG;if(n===Da)return r.COMPRESSED_RGB_PVRTC_2BPPV1_IMG;if(n===La)return r.COMPRESSED_RGBA_PVRTC_4BPPV1_IMG;if(n===Na)return r.COMPRESSED_RGBA_PVRTC_2BPPV1_IMG}else return null;if(n===Ua||n===Fa||n===Oa||n===Ba||n===za||n===Zr||n===ka)if(r=e.get("WEBGL_compressed_texture_etc"),r!==null){if(n===Ua||n===Fa)return o===pt?r.COMPRESSED_SRGB8_ETC2:r.COMPRESSED_RGB8_ETC2;if(n===Oa)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ETC2_EAC:r.COMPRESSED_RGBA8_ETC2_EAC;if(n===Ba)return r.COMPRESSED_R11_EAC;if(n===za)return r.COMPRESSED_SIGNED_R11_EAC;if(n===Zr)return r.COMPRESSED_RG11_EAC;if(n===ka)return r.COMPRESSED_SIGNED_RG11_EAC}else return null;if(n===Va||n===Ga||n===Ha||n===Wa||n===Xa||n===qa||n===Ya||n===Za||n===Ja||n===$a||n===Ka||n===ja||n===Qa||n===el)if(r=e.get("WEBGL_compressed_texture_astc"),r!==null){if(n===Va)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_4x4_KHR:r.COMPRESSED_RGBA_ASTC_4x4_KHR;if(n===Ga)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_5x4_KHR:r.COMPRESSED_RGBA_ASTC_5x4_KHR;if(n===Ha)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_5x5_KHR:r.COMPRESSED_RGBA_ASTC_5x5_KHR;if(n===Wa)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_6x5_KHR:r.COMPRESSED_RGBA_ASTC_6x5_KHR;if(n===Xa)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_6x6_KHR:r.COMPRESSED_RGBA_ASTC_6x6_KHR;if(n===qa)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_8x5_KHR:r.COMPRESSED_RGBA_ASTC_8x5_KHR;if(n===Ya)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_8x6_KHR:r.COMPRESSED_RGBA_ASTC_8x6_KHR;if(n===Za)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_8x8_KHR:r.COMPRESSED_RGBA_ASTC_8x8_KHR;if(n===Ja)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_10x5_KHR:r.COMPRESSED_RGBA_ASTC_10x5_KHR;if(n===$a)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_10x6_KHR:r.COMPRESSED_RGBA_ASTC_10x6_KHR;if(n===Ka)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_10x8_KHR:r.COMPRESSED_RGBA_ASTC_10x8_KHR;if(n===ja)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_10x10_KHR:r.COMPRESSED_RGBA_ASTC_10x10_KHR;if(n===Qa)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_12x10_KHR:r.COMPRESSED_RGBA_ASTC_12x10_KHR;if(n===el)return o===pt?r.COMPRESSED_SRGB8_ALPHA8_ASTC_12x12_KHR:r.COMPRESSED_RGBA_ASTC_12x12_KHR}else return null;if(n===tl||n===nl||n===il)if(r=e.get("EXT_texture_compression_bptc"),r!==null){if(n===tl)return o===pt?r.COMPRESSED_SRGB_ALPHA_BPTC_UNORM_EXT:r.COMPRESSED_RGBA_BPTC_UNORM_EXT;if(n===nl)return r.COMPRESSED_RGB_BPTC_SIGNED_FLOAT_EXT;if(n===il)return r.COMPRESSED_RGB_BPTC_UNSIGNED_FLOAT_EXT}else return null;if(n===sl||n===rl||n===Jr||n===ol)if(r=e.get("EXT_texture_compression_rgtc"),r!==null){if(n===sl)return r.COMPRESSED_RED_RGTC1_EXT;if(n===rl)return r.COMPRESSED_SIGNED_RED_RGTC1_EXT;if(n===Jr)return r.COMPRESSED_RED_GREEN_RGTC2_EXT;if(n===ol)return r.COMPRESSED_SIGNED_RED_GREEN_RGTC2_EXT}else return null;return n===Qi?s.UNSIGNED_INT_24_8:s[n]!==void 0?s[n]:null}return{convert:t}}var gy=`
void main() {

	gl_Position = vec4( position, 1.0 );

}`,_y=`
uniform sampler2DArray depthColor;
uniform float depthWidth;
uniform float depthHeight;

void main() {

	vec2 coord = vec2( gl_FragCoord.x / depthWidth, gl_FragCoord.y / depthHeight );

	if ( coord.x >= 1.0 ) {

		gl_FragDepth = texture( depthColor, vec3( coord.x - 1.0, coord.y, 1 ) ).r;

	} else {

		gl_FragDepth = texture( depthColor, vec3( coord.x, coord.y, 0 ) ).r;

	}

}`,dh=class{constructor(){this.texture=null,this.mesh=null,this.depthNear=0,this.depthFar=0}init(e,t){if(this.texture===null){let n=new wr(e.texture);(e.depthNear!==t.depthNear||e.depthFar!==t.depthFar)&&(this.depthNear=e.depthNear,this.depthFar=e.depthFar),this.texture=n}}getMesh(e){if(this.texture!==null&&this.mesh===null){let t=e.cameras[0].viewport,n=new Vn({vertexShader:gy,fragmentShader:_y,uniforms:{depthColor:{value:this.texture},depthWidth:{value:t.z},depthHeight:{value:t.w}}});this.mesh=new ke(new Xi(20,20),n)}return this.mesh}reset(){this.texture=null,this.mesh=null}getDepthTexture(){return this.texture}},ph=class extends Bn{constructor(e,t){super();let n=this,i=null,r=1,o=null,a="local-floor",l=1,c=null,u=null,h=null,d=null,f=null,m=null,_=typeof XRWebGLBinding<"u",p=new dh,g={},S=t.getContextAttributes(),v=null,x=null,b=[],w=[],y=new pe,M=null,C=null,E=new Ct;E.viewport=new _t;let R=new Ct;R.viewport=new _t;let D=[E,R],k=new xa,P=null,O=null;this.cameraAutoUpdate=!0,this.enabled=!1,this.isPresenting=!1,this.getController=function(q){let ie=b[q];return ie===void 0&&(ie=new Ps,b[q]=ie),ie.getTargetRaySpace()},this.getControllerGrip=function(q){let ie=b[q];return ie===void 0&&(ie=new Ps,b[q]=ie),ie.getGripSpace()},this.getHand=function(q){let ie=b[q];return ie===void 0&&(ie=new Ps,b[q]=ie),ie.getHandSpace()};function G(q){let ie=w.indexOf(q.inputSource);if(ie===-1)return;let ce=b[ie];ce!==void 0&&(ce.update(q.inputSource,q.frame,c||o),ce.dispatchEvent({type:q.type,data:q.inputSource}))}function U(){i.removeEventListener("select",G),i.removeEventListener("selectstart",G),i.removeEventListener("selectend",G),i.removeEventListener("squeeze",G),i.removeEventListener("squeezestart",G),i.removeEventListener("squeezeend",G),i.removeEventListener("end",U),i.removeEventListener("inputsourceschange",ee);for(let q=0;q<b.length;q++){let ie=w[q];ie!==null&&(w[q]=null,b[q].disconnect(ie))}P=null,O=null,p.reset();for(let q in g)delete g[q];if(e.setRenderTarget(v),f=null,d=null,h=null,i=null,x=null,de.stop(),n.isPresenting=!1,e.setPixelRatio(M),e.setSize(y.width,y.height,!1),C!==null){let q=C.camera;q.fov=C.fov,q.zoom=C.zoom,q.updateProjectionMatrix(),C=null}n.dispatchEvent({type:"sessionend"})}this.setFramebufferScaleFactor=function(q){r=q,n.isPresenting===!0&&Xe("WebXRManager: Cannot change framebuffer scale while presenting.")},this.setReferenceSpaceType=function(q){a=q,n.isPresenting===!0&&Xe("WebXRManager: Cannot change reference space type while presenting.")},this.getReferenceSpace=function(){return c||o},this.setReferenceSpace=function(q){c=q},this.getBaseLayer=function(){return d!==null?d:f},this.getBinding=function(){return h===null&&_&&(h=new XRWebGLBinding(i,t)),h},this.getFrame=function(){return m},this.getSession=function(){return i},this.setSession=async function(q){if(i=q,i!==null){if(v=e.getRenderTarget(),i.addEventListener("select",G),i.addEventListener("selectstart",G),i.addEventListener("selectend",G),i.addEventListener("squeeze",G),i.addEventListener("squeezestart",G),i.addEventListener("squeezeend",G),i.addEventListener("end",U),i.addEventListener("inputsourceschange",ee),S.xrCompatible!==!0&&await t.makeXRCompatible(),M=e.getPixelRatio(),e.getSize(y),_&&"createProjectionLayer"in XRWebGLBinding.prototype){let ce=null,Ce=null,ue=null;S.depth&&(ue=S.stencil?t.DEPTH24_STENCIL8:t.DEPTH_COMPONENT24,ce=S.stencil?di:ii,Ce=S.stencil?Qi:Pn);let H={colorFormat:t.RGBA8,depthFormat:ue,scaleFactor:r};h=this.getBinding(),d=h.createProjectionLayer(H),i.updateRenderState({layers:[d]}),e.setPixelRatio(1),e.setSize(d.textureWidth,d.textureHeight,!1),x=new hn(d.textureWidth,d.textureHeight,{format:Mn,type:on,depthTexture:new oi(d.textureWidth,d.textureHeight,Ce,void 0,void 0,void 0,void 0,void 0,void 0,ce),stencilBuffer:S.stencil,colorSpace:e.outputColorSpace,samples:S.antialias?4:0,resolveDepthBuffer:d.ignoreDepthValues===!1,resolveStencilBuffer:d.ignoreDepthValues===!1,storeMultisampledDepthBuffer:d.ignoreDepthValues===!1,storeMultisampledStencilBuffer:d.ignoreDepthValues===!1})}else{let ce={antialias:S.antialias,alpha:!0,depth:S.depth,stencil:S.stencil,framebufferScaleFactor:r};f=new XRWebGLLayer(i,t,ce),i.updateRenderState({baseLayer:f}),e.setPixelRatio(1),e.setSize(f.framebufferWidth,f.framebufferHeight,!1),x=new hn(f.framebufferWidth,f.framebufferHeight,{format:Mn,type:on,colorSpace:e.outputColorSpace,stencilBuffer:S.stencil,resolveDepthBuffer:f.ignoreDepthValues===!1,resolveStencilBuffer:f.ignoreDepthValues===!1,storeMultisampledDepthBuffer:f.ignoreDepthValues===!1,storeMultisampledStencilBuffer:f.ignoreDepthValues===!1})}x.isXRRenderTarget=!0,this.setFoveation(l),c=null,o=await i.requestReferenceSpace(a),de.setContext(i),de.start(),n.isPresenting=!0,n.dispatchEvent({type:"sessionstart"})}},this.getEnvironmentBlendMode=function(){if(i!==null)return i.environmentBlendMode},this.getDepthTexture=function(){return p.getDepthTexture()};function ee(q){for(let ie=0;ie<q.removed.length;ie++){let ce=q.removed[ie],Ce=w.indexOf(ce);Ce>=0&&(w[Ce]=null,b[Ce].disconnect(ce))}for(let ie=0;ie<q.added.length;ie++){let ce=q.added[ie],Ce=w.indexOf(ce);if(Ce===-1){for(let H=0;H<b.length;H++)if(H>=w.length){w.push(ce),Ce=H;break}else if(w[H]===null){w[H]=ce,Ce=H;break}if(Ce===-1)break}let ue=b[Ce];ue&&ue.connect(ce)}}function W(q,ie){ie===null?q.matrixWorld.copy(q.matrix):q.matrixWorld.multiplyMatrices(ie.matrixWorld,q.matrix),q.matrixWorldInverse.copy(q.matrixWorld).invert()}this.updateCamera=function(q){if(i===null)return;let ie=q.near,ce=q.far;p.texture!==null&&(p.depthNear>0&&(ie=p.depthNear),p.depthFar>0&&(ce=p.depthFar)),k.near=R.near=E.near=ie,k.far=R.far=E.far=ce,(P!==k.near||O!==k.far)&&(i.updateRenderState({depthNear:k.near,depthFar:k.far}),P=k.near,O=k.far),k.layers.mask=q.layers.mask|6,E.layers.mask=k.layers.mask&-5,R.layers.mask=k.layers.mask&-3;let Ce=q.parent,ue=k.cameras;W(k,Ce);for(let H=0;H<ue.length;H++)W(ue[H],Ce);k.projectionMatrix.copy(E.projectionMatrix),k.projectionMatrixInverse.copy(E.projectionMatrixInverse),C===null&&q.isPerspectiveCamera&&(C={camera:q,fov:q.fov,zoom:q.zoom}),K(q,k,Ce)};function K(q,ie,ce){ce===null?q.matrix.copy(ie.matrixWorld):(q.matrix.copy(ce.matrixWorld),q.matrix.invert(),q.matrix.multiply(ie.matrixWorld)),q.matrix.decompose(q.position,q.quaternion,q.scale),q.updateMatrixWorld(!0),q.projectionMatrix.copy(ie.projectionMatrix),q.projectionMatrixInverse.copy(ie.projectionMatrixInverse),q.isPerspectiveCamera&&(q.fov=mr*2*Math.atan(1/q.projectionMatrix.elements[5]),q.zoom=1)}this.getCamera=function(){return k},this.getFoveation=function(){if(!(d===null&&f===null))return l},this.setFoveation=function(q){l=q,d!==null&&(d.fixedFoveation=q),f!==null&&f.fixedFoveation!==void 0&&(f.fixedFoveation=q)},this.hasDepthSensing=function(){return p.texture!==null},this.getDepthSensingMesh=function(){return p.getMesh(k)},this.getCameraTexture=function(q){return g[q]};let $=null;function he(q,ie){if(u=ie.getViewerPose(c||o),m=ie,u!==null){let ce=u.views;f!==null&&(e.setRenderTargetFramebuffer(x,f.framebuffer),e.setRenderTarget(x));let Ce=!1;ce.length!==k.cameras.length&&(k.cameras.length=0,Ce=!0);for(let fe=0;fe<ce.length;fe++){let Le=ce[fe],V=null;if(f!==null)V=f.getViewport(Le);else{let X=h.getViewSubImage(d,Le);V=X.viewport,fe===0&&(e.setRenderTargetTextures(x,X.colorTexture,X.depthStencilTexture),e.setRenderTarget(x))}let N=D[fe];N===void 0&&(N=new Ct,N.layers.enable(fe),N.viewport=new _t,D[fe]=N),N.matrix.fromArray(Le.transform.matrix),N.matrix.decompose(N.position,N.quaternion,N.scale),N.projectionMatrix.fromArray(Le.projectionMatrix),N.projectionMatrixInverse.copy(N.projectionMatrix).invert(),N.viewport.set(V.x,V.y,V.width,V.height),fe===0&&(k.matrix.copy(N.matrix),k.matrix.decompose(k.position,k.quaternion,k.scale)),Ce===!0&&k.cameras.push(N)}let ue=i.enabledFeatures;if(ue&&ue.includes("depth-sensing")&&i.depthUsage=="gpu-optimized"&&_){h=n.getBinding();let fe=h.getDepthInformation(ce[0]);fe&&fe.isValid&&fe.texture&&p.init(fe,i.renderState)}if(ue&&ue.includes("camera-access")&&_){e.state.unbindTexture(),h=n.getBinding();for(let fe=0;fe<ce.length;fe++){let Le=ce[fe].camera;if(Le){let V=g[Le];V||(V=new wr,g[Le]=V);let N=h.getCameraImage(Le);V.sourceTexture=N}}}}for(let ce=0;ce<b.length;ce++){let Ce=w[ce],ue=b[ce];Ce!==null&&ue!==void 0&&ue.update(Ce,ie,c||o)}$&&$(q,ie),ie.detectedPlanes&&n.dispatchEvent({type:"planesdetected",data:ie}),m=null}let de=new id;de.setAnimationLoop(he),this.setAnimationLoop=function(q){$=q},this.dispose=function(){}}},xy=new ze,dd=new Ze;dd.set(-1,0,0,0,1,0,0,0,1);function yy(s,e){function t(p,g){p.matrixAutoUpdate===!0&&p.updateMatrix(),g.value.copy(p.matrix)}function n(p,g){g.color.getRGB(p.fogColor.value,$c(s)),g.isFog?(p.fogNear.value=g.near,p.fogFar.value=g.far):g.isFogExp2&&(p.fogDensity.value=g.density)}function i(p,g,S,v,x){g.isNodeMaterial?g.uniformsNeedUpdate=!1:g.isMeshBasicMaterial?r(p,g):g.isMeshLambertMaterial?(r(p,g),g.envMap&&(p.envMapIntensity.value=g.envMapIntensity)):g.isMeshToonMaterial?(r(p,g),h(p,g)):g.isMeshPhongMaterial?(r(p,g),u(p,g),g.envMap&&(p.envMapIntensity.value=g.envMapIntensity)):g.isMeshStandardMaterial?(r(p,g),d(p,g),g.isMeshPhysicalMaterial&&f(p,g,x)):g.isMeshMatcapMaterial?(r(p,g),m(p,g)):g.isMeshDepthMaterial?r(p,g):g.isMeshDistanceMaterial?(r(p,g),_(p,g)):g.isMeshNormalMaterial?r(p,g):g.isLineBasicMaterial?(o(p,g),g.isLineDashedMaterial&&a(p,g)):g.isPointsMaterial?l(p,g,S,v):g.isSpriteMaterial?c(p,g):g.isShadowMaterial?(p.color.value.copy(g.color),p.opacity.value=g.opacity):g.isShaderMaterial&&(g.uniformsNeedUpdate=!1)}function r(p,g){p.opacity.value=g.opacity,g.color&&p.diffuse.value.copy(g.color),g.emissive&&p.emissive.value.copy(g.emissive).multiplyScalar(g.emissiveIntensity),g.map&&(p.map.value=g.map,t(g.map,p.mapTransform)),g.alphaMap&&(p.alphaMap.value=g.alphaMap,t(g.alphaMap,p.alphaMapTransform)),g.bumpMap&&(p.bumpMap.value=g.bumpMap,t(g.bumpMap,p.bumpMapTransform),p.bumpScale.value=g.bumpScale,g.side===Xt&&(p.bumpScale.value*=-1)),g.normalMap&&(p.normalMap.value=g.normalMap,t(g.normalMap,p.normalMapTransform),p.normalScale.value.copy(g.normalScale),g.side===Xt&&p.normalScale.value.negate()),g.displacementMap&&(p.displacementMap.value=g.displacementMap,t(g.displacementMap,p.displacementMapTransform),p.displacementScale.value=g.displacementScale,p.displacementBias.value=g.displacementBias),g.emissiveMap&&(p.emissiveMap.value=g.emissiveMap,t(g.emissiveMap,p.emissiveMapTransform)),g.specularMap&&(p.specularMap.value=g.specularMap,t(g.specularMap,p.specularMapTransform)),g.alphaTest>0&&(p.alphaTest.value=g.alphaTest);let S=e.get(g),v=S.envMap,x=S.envMapRotation;v&&(p.envMap.value=v,p.envMapRotation.value.setFromMatrix4(xy.makeRotationFromEuler(x)).transpose(),v.isCubeTexture&&v.isRenderTargetTexture===!1&&p.envMapRotation.value.premultiply(dd),p.reflectivity.value=g.reflectivity,p.ior.value=g.ior,p.refractionRatio.value=g.refractionRatio),g.lightMap&&(p.lightMap.value=g.lightMap,p.lightMapIntensity.value=g.lightMapIntensity,t(g.lightMap,p.lightMapTransform)),g.aoMap&&(p.aoMap.value=g.aoMap,p.aoMapIntensity.value=g.aoMapIntensity,t(g.aoMap,p.aoMapTransform))}function o(p,g){p.diffuse.value.copy(g.color),p.opacity.value=g.opacity,g.map&&(p.map.value=g.map,t(g.map,p.mapTransform))}function a(p,g){p.dashSize.value=g.dashSize,p.totalSize.value=g.dashSize+g.gapSize,p.scale.value=g.scale}function l(p,g,S,v){p.diffuse.value.copy(g.color),p.opacity.value=g.opacity,p.size.value=g.size*S,p.scale.value=v*.5,g.map&&(p.map.value=g.map,t(g.map,p.uvTransform)),g.alphaMap&&(p.alphaMap.value=g.alphaMap,t(g.alphaMap,p.alphaMapTransform)),g.alphaTest>0&&(p.alphaTest.value=g.alphaTest)}function c(p,g){p.diffuse.value.copy(g.color),p.opacity.value=g.opacity,p.rotation.value=g.rotation,g.map&&(p.map.value=g.map,t(g.map,p.mapTransform)),g.alphaMap&&(p.alphaMap.value=g.alphaMap,t(g.alphaMap,p.alphaMapTransform)),g.alphaTest>0&&(p.alphaTest.value=g.alphaTest)}function u(p,g){p.specular.value.copy(g.specular),p.shininess.value=Math.max(g.shininess,1e-4)}function h(p,g){g.gradientMap&&(p.gradientMap.value=g.gradientMap)}function d(p,g){p.metalness.value=g.metalness,g.metalnessMap&&(p.metalnessMap.value=g.metalnessMap,t(g.metalnessMap,p.metalnessMapTransform)),p.roughness.value=g.roughness,g.roughnessMap&&(p.roughnessMap.value=g.roughnessMap,t(g.roughnessMap,p.roughnessMapTransform)),g.envMap&&(p.envMapIntensity.value=g.envMapIntensity)}function f(p,g,S){p.ior.value=g.ior,g.diffuseRoughness>0&&(p.diffuseRoughness.value=g.diffuseRoughness,g.diffuseRoughnessMap&&(p.diffuseRoughnessMap.value=g.diffuseRoughnessMap,t(g.diffuseRoughnessMap,p.diffuseRoughnessMapTransform))),g.sheen>0&&(p.sheenColor.value.copy(g.sheenColor).multiplyScalar(g.sheen),p.sheenRoughness.value=g.sheenRoughness,g.sheenColorMap&&(p.sheenColorMap.value=g.sheenColorMap,t(g.sheenColorMap,p.sheenColorMapTransform)),g.sheenRoughnessMap&&(p.sheenRoughnessMap.value=g.sheenRoughnessMap,t(g.sheenRoughnessMap,p.sheenRoughnessMapTransform))),g.clearcoat>0&&(p.clearcoat.value=g.clearcoat,p.clearcoatRoughness.value=g.clearcoatRoughness,g.clearcoatMap&&(p.clearcoatMap.value=g.clearcoatMap,t(g.clearcoatMap,p.clearcoatMapTransform)),g.clearcoatRoughnessMap&&(p.clearcoatRoughnessMap.value=g.clearcoatRoughnessMap,t(g.clearcoatRoughnessMap,p.clearcoatRoughnessMapTransform)),g.clearcoatNormalMap&&(p.clearcoatNormalMap.value=g.clearcoatNormalMap,t(g.clearcoatNormalMap,p.clearcoatNormalMapTransform),p.clearcoatNormalScale.value.copy(g.clearcoatNormalScale),g.side===Xt&&p.clearcoatNormalScale.value.negate())),g.dispersion>0&&(p.dispersion.value=g.dispersion),g.retroreflectivity>0&&(p.retroreflectivity.value=g.retroreflectivity),g.iridescence>0&&(p.iridescence.value=g.iridescence,p.iridescenceIOR.value=g.iridescenceIOR,p.iridescenceThicknessMinimum.value=g.iridescenceThicknessMinimum,p.iridescenceThicknessMaximum.value=g.iridescenceThicknessMaximum,g.iridescenceMap&&(p.iridescenceMap.value=g.iridescenceMap,t(g.iridescenceMap,p.iridescenceMapTransform)),g.iridescenceThicknessMap&&(p.iridescenceThicknessMap.value=g.iridescenceThicknessMap,t(g.iridescenceThicknessMap,p.iridescenceThicknessMapTransform))),g.transmission>0&&(p.transmission.value=g.transmission,p.transmissionSamplerMap.value=S.texture,p.transmissionSamplerSize.value.set(S.width,S.height),g.transmissionMap&&(p.transmissionMap.value=g.transmissionMap,t(g.transmissionMap,p.transmissionMapTransform)),p.thickness.value=g.thickness,g.thicknessMap&&(p.thicknessMap.value=g.thicknessMap,t(g.thicknessMap,p.thicknessMapTransform)),p.attenuationDistance.value=g.attenuationDistance,p.attenuationColor.value.copy(g.attenuationColor)),g.anisotropy>0&&(p.anisotropyVector.value.set(g.anisotropy*Math.cos(g.anisotropyRotation),g.anisotropy*Math.sin(g.anisotropyRotation)),g.anisotropyMap&&(p.anisotropyMap.value=g.anisotropyMap,t(g.anisotropyMap,p.anisotropyMapTransform))),p.specularIntensity.value=g.specularIntensity,p.specularColor.value.copy(g.specularColor),g.specularColorMap&&(p.specularColorMap.value=g.specularColorMap,t(g.specularColorMap,p.specularColorMapTransform)),g.specularIntensityMap&&(p.specularIntensityMap.value=g.specularIntensityMap,t(g.specularIntensityMap,p.specularIntensityMapTransform))}function m(p,g){g.matcap&&(p.matcap.value=g.matcap)}function _(p,g){let S=e.get(g).light;p.referencePosition.value.setFromMatrixPosition(S.matrixWorld),p.nearDistance.value=S.shadow.camera.near,p.farDistance.value=S.shadow.camera.far}return{refreshFogUniforms:n,refreshMaterialUniforms:i}}function vy(s,e,t,n){let i={},r={};function o(S,v){let x=v.program;n.uniformBlockBinding(S,x,i[S.id])}function a(S,v){let x=i[S.id];x===void 0&&(m(S),x=l(S),i[S.id]=x,S.addEventListener("dispose",p));let b=v.program;n.updateUBOMapping(S,b);let w=e.render.frame;r[S.id]!==w&&(c(S),r[S.id]=w)}function l(S){let v=s.createBuffer(),x=S.__size,b=S.usage;return s.bindBuffer(s.UNIFORM_BUFFER,v),s.bufferData(s.UNIFORM_BUFFER,x,b),s.bindBuffer(s.UNIFORM_BUFFER,null),v}function c(S){let v=i[S.id],x=S.uniforms,b=S.__cache;s.bindBuffer(s.UNIFORM_BUFFER,v);for(let w=0,y=x.length;w<y;w++){let M=x[w];if(Array.isArray(M))for(let C=0,E=M.length;C<E;C++)u(M[C],w,C,b);else u(M,w,0,b)}s.bindBuffer(s.UNIFORM_BUFFER,null)}function u(S,v,x,b){if(d(S,v,x,b)===!0){let w=S.__offset,y=S.value;if(Array.isArray(y)){let M=0;for(let C=0;C<y.length;C++){let E=y[C],R=_(E,!0);h(E,S.__data,M),M+=R.storage/Float32Array.BYTES_PER_ELEMENT}}else h(y,S.__data,0);s.bufferSubData(s.UNIFORM_BUFFER,w,S.__data)}}function h(S,v,x){typeof S=="number"||typeof S=="boolean"?v[x]=S:S.isMatrix3?(v[x]=S.elements[0],v[x+1]=S.elements[1],v[x+2]=S.elements[2],v[x+3]=0,v[x+4]=S.elements[3],v[x+5]=S.elements[4],v[x+6]=S.elements[5],v[x+7]=0,v[x+8]=S.elements[6],v[x+9]=S.elements[7],v[x+10]=S.elements[8],v[x+11]=0):ArrayBuffer.isView(S)?v.set(S.subarray(0,v.length-x),x):S.toArray(v,x)}function d(S,v,x,b){let w=S.value,y=v+"_"+x;if(Array.isArray(w)){b[y]===void 0&&(b[y]=[]);let M=b[y],C=!1;for(let E=0;E<w.length;E++)f(w[E],M,E)===!0&&(C=!0);return C}return f(w,b,y)}function f(S,v,x){if(v[x]===void 0)return typeof S=="number"||typeof S=="boolean"?v[x]=S:ArrayBuffer.isView(S)?v[x]=S.slice():v[x]=S.clone(),!0;{let b=v[x];if(typeof S=="number"||typeof S=="boolean"){if(b!==S)return v[x]=S,!0}else{if(ArrayBuffer.isView(S))return!0;if(b.equals(S)===!1)return b.copy(S),!0}}return!1}function m(S){let v=S.uniforms,x=0,b=16;for(let y=0,M=v.length;y<M;y++){let C=Array.isArray(v[y])?v[y]:[v[y]];for(let E=0,R=C.length;E<R;E++){let D=C[E],k=Array.isArray(D.value)?D.value:[D.value],P=Array.isArray(v[y])||Array.isArray(D.value),O=x;for(let U=0,ee=k.length;U<ee;U++){let W=k[U],K=_(W,P),$=x%b,he=$%K.boundary,de=$+he;x+=he,de!==0&&b-de<K.storage&&(x+=b-de),U===0&&(O=x),x+=K.storage}let G=D.type==="int"?Int32Array:D.type==="uint"?Uint32Array:Float32Array;D.__data=new G((x-O)/Float32Array.BYTES_PER_ELEMENT),D.__offset=O}}let w=x%b;return w>0&&(x+=b-w),S.__size=x,S.__cache={},this}function _(S,v){let x={boundary:0,storage:0};return typeof S=="number"||typeof S=="boolean"?(x.boundary=4,x.storage=4):S.isVector2?(x.boundary=8,x.storage=8):S.isVector3||S.isColor?(x.boundary=16,x.storage=12):S.isVector4?(x.boundary=16,x.storage=16):S.isMatrix3?(x.boundary=16,x.storage=48):S.isMatrix4?(x.boundary=16,x.storage=64):S.isTexture?Xe("WebGLRenderer: Texture samplers can not be part of an uniforms group."):ArrayBuffer.isView(S)?(x.boundary=16,x.storage=S.byteLength):Xe("WebGLRenderer: Unsupported uniform value type.",S),v===!0&&(x.boundary=16,x.storage=Math.ceil(x.storage/16)*16),x}function p(S){let v=S.target;v.removeEventListener("dispose",p),s.deleteBuffer(i[v.id]),delete i[v.id],delete r[v.id]}function g(){for(let S in i)s.deleteBuffer(i[S]);i={},r={}}return{bind:o,update:a,dispose:g}}var Sy=new Uint16Array([12469,15057,12620,14925,13266,14620,13807,14376,14323,13990,14545,13625,14713,13328,14840,12882,14931,12528,14996,12233,15039,11829,15066,11525,15080,11295,15085,10976,15082,10705,15073,10495,13880,14564,13898,14542,13977,14430,14158,14124,14393,13732,14556,13410,14702,12996,14814,12596,14891,12291,14937,11834,14957,11489,14958,11194,14943,10803,14921,10506,14893,10278,14858,9960,14484,14039,14487,14025,14499,13941,14524,13740,14574,13468,14654,13106,14743,12678,14818,12344,14867,11893,14889,11509,14893,11180,14881,10751,14852,10428,14812,10128,14765,9754,14712,9466,14764,13480,14764,13475,14766,13440,14766,13347,14769,13070,14786,12713,14816,12387,14844,11957,14860,11549,14868,11215,14855,10751,14825,10403,14782,10044,14729,9651,14666,9352,14599,9029,14967,12835,14966,12831,14963,12804,14954,12723,14936,12564,14917,12347,14900,11958,14886,11569,14878,11247,14859,10765,14828,10401,14784,10011,14727,9600,14660,9289,14586,8893,14508,8533,15111,12234,15110,12234,15104,12216,15092,12156,15067,12010,15028,11776,14981,11500,14942,11205,14902,10752,14861,10393,14812,9991,14752,9570,14682,9252,14603,8808,14519,8445,14431,8145,15209,11449,15208,11451,15202,11451,15190,11438,15163,11384,15117,11274,15055,10979,14994,10648,14932,10343,14871,9936,14803,9532,14729,9218,14645,8742,14556,8381,14461,8020,14365,7603,15273,10603,15272,10607,15267,10619,15256,10631,15231,10614,15182,10535,15118,10389,15042,10167,14963,9787,14883,9447,14800,9115,14710,8665,14615,8318,14514,7911,14411,7507,14279,7198,15314,9675,15313,9683,15309,9712,15298,9759,15277,9797,15229,9773,15166,9668,15084,9487,14995,9274,14898,8910,14800,8539,14697,8234,14590,7790,14479,7409,14367,7067,14178,6621,15337,8619,15337,8631,15333,8677,15325,8769,15305,8871,15264,8940,15202,8909,15119,8775,15022,8565,14916,8328,14804,8009,14688,7614,14569,7287,14448,6888,14321,6483,14088,6171,15350,7402,15350,7419,15347,7480,15340,7613,15322,7804,15287,7973,15229,8057,15148,8012,15046,7846,14933,7611,14810,7357,14682,7069,14552,6656,14421,6316,14251,5948,14007,5528,15356,5942,15356,5977,15353,6119,15348,6294,15332,6551,15302,6824,15249,7044,15171,7122,15070,7050,14949,6861,14818,6611,14679,6349,14538,6067,14398,5651,14189,5311,13935,4958,15359,4123,15359,4153,15356,4296,15353,4646,15338,5160,15311,5508,15263,5829,15188,6042,15088,6094,14966,6001,14826,5796,14678,5543,14527,5287,14377,4985,14133,4586,13869,4257,15360,1563,15360,1642,15358,2076,15354,2636,15341,3350,15317,4019,15273,4429,15203,4732,15105,4911,14981,4932,14836,4818,14679,4621,14517,4386,14359,4156,14083,3795,13808,3437,15360,122,15360,137,15358,285,15355,636,15344,1274,15322,2177,15281,2765,15215,3223,15120,3451,14995,3569,14846,3567,14681,3466,14511,3305,14344,3121,14037,2800,13753,2467,15360,0,15360,1,15359,21,15355,89,15346,253,15325,479,15287,796,15225,1148,15133,1492,15008,1749,14856,1882,14685,1886,14506,1783,14324,1608,13996,1398,13702,1183]),Wn=null;function My(){return Wn===null&&(Wn=new Ls(Sy,16,16,Ci,In),Wn.name="DFG_LUT",Wn.minFilter=Dt,Wn.magFilter=Dt,Wn.wrapS=_n,Wn.wrapT=_n,Wn.generateMipmaps=!1,Wn.needsUpdate=!0),Wn}var Zs=class{constructor(e={}){let{canvas:t=bf(),context:n=null,depth:i=!0,stencil:r=!1,alpha:o=!1,antialias:a=!1,premultipliedAlpha:l=!0,preserveDrawingBuffer:c=!1,powerPreference:u="default",failIfMajorPerformanceCaveat:h=!1,reversedDepthBuffer:d=!1,outputBufferType:f=on}=e;this.isWebGLRenderer=!0;let m;if(n!==null){if(typeof WebGLRenderingContext<"u"&&n instanceof WebGLRenderingContext)throw new Error("THREE.WebGLRenderer: WebGL 1 is not supported since r163.");m=n.getContextAttributes().alpha}else m=o;let _=f,p=new Set([Pa,Ca,Ra]),g=new Set([on,Pn,Xs,Qi,Aa,Ta]),S=new Uint32Array(4),v=new Int32Array(4),x={textures:[],min:[],max:[],resolution:[]},b=null,w=null,y=[],M=[],C=null;this.domElement=t,this.debug={checkShaderErrors:!0,diagnostics:{keywords:!1},onShaderError:null},this.autoClear=!0,this.autoClearColor=!0,this.autoClearDepth=!0,this.autoClearStencil=!0,this.sortObjects=!0,this.clippingPlanes=[],this.localClippingEnabled=!1,this.toneMapping=Hn,this.toneMappingExposure=1,this.transmissionResolutionScale=1;let E=this,R=!1,D=null,k=null,P=null,O=null;this._outputColorSpace=wt;let G=0,U=0,ee=null,W=-1,K=null,$=new _t,he=new _t,de=null,q=new Ke(0),ie=0,ce=t.width,Ce=t.height,ue=1,H=null,se=null,fe=new _t(0,0,ce,Ce),Le=new _t(0,0,ce,Ce),V=!1,N=new wi,X=new na,j=!1,ge=!1,le=new ze,Re=new L,xe=new _t,Ue={background:null,fog:null,environment:null,overrideMaterial:null,isScene:!0},Je=!1;function Qe(){return ee===null?ue:1}let B=n;function xt(T,z){return t.getContext(T,z)}let je,rt,Me,yt,He,qe,I,A,Z,me,ae,ve,Te,ne,re,be,F,oe,te,we,ye,Se,Ge;try{let T={alpha:!0,depth:i,stencil:r,antialias:a,premultipliedAlpha:l,preserveDrawingBuffer:c,powerPreference:u,failIfMajorPerformanceCaveat:h};if("setAttribute"in t&&t.setAttribute("data-engine",`three.js r${"187"}`),t.addEventListener("webglcontextlost",We,!1),t.addEventListener("webglcontextrestored",et,!1),t.addEventListener("webglcontextcreationerror",Pe,!1),B===null){let z="webgl2";if(B=xt(z,T),B===null)throw xt(z)?new Error("THREE.WebGLRenderer: Error creating WebGL context with your selected attributes."):new Error("THREE.WebGLRenderer: Error creating WebGL context.")}tt()}catch(T){throw t.removeEventListener("webglcontextlost",We,!1),t.removeEventListener("webglcontextrestored",et,!1),t.removeEventListener("webglcontextcreationerror",Pe,!1),Ye("WebGLRenderer: "+T.message),T}function tt(){je=new L_(B),je.init(),ye=new my(B,je),rt=new M_(B,je,e,ye),Me=new dy(B,je),rt.reversedDepthBuffer&&d&&Me.buffers.depth.setReversed(!0),k=B.createFramebuffer(),P=B.createFramebuffer(),O=B.createFramebuffer(),yt=new F_(B),He=new Qx,qe=new py(B,je,Me,He,rt,ye,yt),I=new D_(E),A=new Bm(B),Se=new v_(B,A),Z=new N_(B,A,yt,Se),me=new B_(B,Z,A,Se,yt),oe=new O_(B,rt,qe),re=new b_(He),ae=new jx(E,I,je,rt,Se,re),ve=new yy(E,He),Te=new ty,ne=new ay(je),F=new y_(E,I,Me,me,m,l),be=new fy(E,me,rt),Ge=new vy(B,yt,rt,Me),te=new S_(B,je,yt),we=new U_(B,je,yt),yt.programs=ae.programs,E.capabilities=rt,E.extensions=je,E.properties=He,E.renderLists=Te,E.shadowMap=be,E.state=Me,E.info=yt}_!==on&&(C=new k_(_,t.width,t.height,a,i,r));let _e=new ph(E,B);this.xr=_e,this.getContext=function(){return B},this.getContextAttributes=function(){return B.getContextAttributes()},this.forceContextLoss=function(){let T=je.get("WEBGL_lose_context");T&&T.loseContext()},this.forceContextRestore=function(){let T=je.get("WEBGL_lose_context");T&&T.restoreContext()},this.getPixelRatio=function(){return ue},this.setPixelRatio=function(T){T!==void 0&&(ue=T,this.setSize(ce,Ce,!1))},this.getSize=function(T){return T.set(ce,Ce)},this.setSize=function(T,z,Q=!0){if(_e.isPresenting){Xe("WebGLRenderer: Can't change size while VR device is presenting.");return}ce=T,Ce=z,t.width=Math.floor(T*ue),t.height=Math.floor(z*ue),Q===!0&&(t.style.width=T+"px",t.style.height=z+"px"),C!==null&&C.setSize(t.width,t.height),this.setViewport(0,0,T,z)},this.getDrawingBufferSize=function(T){return T.set(ce*ue,Ce*ue).floor()},this.setDrawingBufferSize=function(T,z,Q){ce=T,Ce=z,ue=Q,t.width=Math.floor(T*Q),t.height=Math.floor(z*Q),this.setViewport(0,0,T,z)},this.setEffects=function(T){if(_===on){Ye("WebGLRenderer: setEffects() requires outputBufferType set to HalfFloatType or FloatType.");return}if(T){for(let z=0;z<T.length;z++)if(T[z].isOutputPass===!0){Xe("WebGLRenderer: OutputPass is not needed in setEffects(). Tone mapping and color space conversion are applied automatically.");break}}C.setEffects(T||[])},this.getCurrentViewport=function(T){return T.copy($)},this.getViewport=function(T){return T.copy(fe)},this.setViewport=function(T,z,Q,Y){T.isVector4?fe.set(T.x,T.y,T.z,T.w):fe.set(T,z,Q,Y),Me.viewport($.copy(fe).multiplyScalar(Qe()).round())},this.getScissor=function(T){return T.copy(Le)},this.setScissor=function(T,z,Q,Y){T.isVector4?Le.set(T.x,T.y,T.z,T.w):Le.set(T,z,Q,Y),Me.scissor(he.copy(Le).multiplyScalar(Qe()).round())},this.getScissorTest=function(){return V},this.setScissorTest=function(T){Me.setScissorTest(V=T)},this.setOpaqueSort=function(T){H=T},this.setTransparentSort=function(T){se=T},this.getClearColor=function(T){return T.copy(F.getClearColor())},this.setClearColor=function(){F.setClearColor(...arguments)},this.getClearAlpha=function(){return F.getClearAlpha()},this.setClearAlpha=function(){F.setClearAlpha(...arguments)},this.clear=function(T=!0,z=!0,Q=!0){let Y=0;if(T){let J=!1;if(ee!==null){let Ee=ee.texture.format;J=p.has(Ee)}if(J){let Ee=ee.texture.type,Fe=g.has(Ee),Ae=F.getClearColor(),Ne=F.getClearAlpha(),Ve=Ae.r,$e=Ae.g,ot=Ae.b;Fe?(S[0]=Ve,S[1]=$e,S[2]=ot,S[3]=Ne,B.clearBufferuiv(B.COLOR,0,S)):(v[0]=Ve,v[1]=$e,v[2]=ot,v[3]=Ne,B.clearBufferiv(B.COLOR,0,v))}else Y|=B.COLOR_BUFFER_BIT}z&&(Y|=B.DEPTH_BUFFER_BIT,this.state.buffers.depth.setMask(!0)),Q&&(Y|=B.STENCIL_BUFFER_BIT,this.state.buffers.stencil.setMask(4294967295)),Y!==0&&B.clear(Y)},this.clearColor=function(){this.clear(!0,!1,!1)},this.clearDepth=function(){this.clear(!1,!0,!1)},this.clearStencil=function(){this.clear(!1,!1,!0)},this.setNodesHandler=function(T){T.setRenderer(this),D=T},this.dispose=function(){t.removeEventListener("webglcontextlost",We,!1),t.removeEventListener("webglcontextrestored",et,!1),t.removeEventListener("webglcontextcreationerror",Pe,!1),F.dispose(),Te.dispose(),ne.dispose(),qe.dispose(),He.dispose(),I.dispose(),me.dispose(),Z.dispose(),Se.dispose(),Ge.dispose(),ae.dispose(),_e.dispose(),_e.removeEventListener("sessionstart",Wh),_e.removeEventListener("sessionend",Xh),Ni.stop()};function We(T){T.preventDefault(),Yc("WebGLRenderer: Context Lost."),R=!0}function et(){Yc("WebGLRenderer: Context Restored."),R=!1;let T=yt.autoReset,z=be.enabled,Q=be.autoUpdate,Y=be.needsUpdate,J=be.type;tt(),yt.autoReset=T,be.enabled=z,be.autoUpdate=Q,be.needsUpdate=Y,be.type=J}function Pe(T){Ye("WebGLRenderer: A WebGL context could not be created. Reason: ",T.statusMessage)}function mt(T){let z=T.target;z.removeEventListener("dispose",mt),Bt(z)}function Bt(T){gi(T),He.remove(T)}function gi(T){let z=He.get(T).programs;z!==void 0&&(z.forEach(function(Q){ae.releaseProgram(Q)}),T.isShaderMaterial&&ae.releaseShaderCache(T))}this.renderBufferDirect=function(T,z,Q,Y,J,Ee){z===null&&(z=Ue);let Fe=J.isMesh&&J.matrixWorld.determinantAffine()<0,Ae=up(T,z,Q,Y,J);Me.setMaterial(Y,Fe);let Ne=Q.index,Ve=1;if(Y.wireframe===!0){if(Ne=Z.getWireframeAttribute(Q),Ne===void 0)return;Ve=2}let $e=Q.drawRange,ot=Q.attributes.position,Oe=$e.start*Ve,at=($e.start+$e.count)*Ve;Ee!==null&&(Oe=Math.max(Oe,Ee.start*Ve),at=Math.min(at,(Ee.start+Ee.count)*Ve)),Ne!==null?(Oe=Math.max(Oe,0),at=Math.min(at,Ne.count)):ot!=null&&(Oe=Math.max(Oe,0),at=Math.min(at,ot.count));let Pt=at-Oe;if(Pt<0||Pt===1/0)return;Se.setup(J,Y,Ae,Q,Ne);let St,vt=te;if(Ne!==null&&(St=A.get(Ne),vt=we,vt.setIndex(St)),J.isMesh)Y.wireframe===!0?(Me.setLineWidth(Y.wireframeLinewidth*Qe()),vt.setMode(B.LINES)):vt.setMode(B.TRIANGLES);else if(J.isLine){let Zt=Y.linewidth;Zt===void 0&&(Zt=1),Me.setLineWidth(Zt*Qe()),J.isLineSegments?vt.setMode(B.LINES):J.isLineLoop?vt.setMode(B.LINE_LOOP):vt.setMode(B.LINE_STRIP)}else J.isPoints?vt.setMode(B.POINTS):J.isSprite&&vt.setMode(B.TRIANGLES);if(J.isBatchedMesh)if(je.get("WEBGL_multi_draw"))vt.renderMultiDraw(J._multiDrawStarts,J._multiDrawCounts,J._multiDrawCount);else{let Zt=J._multiDrawStarts,Be=J._multiDrawCounts,en=J._multiDrawCount,ft=Ne?A.get(Ne).bytesPerElement:1,Ui=He.get(Y).currentProgram.getUniforms();for(let Jt=0;Jt<en;Jt++)Ui.setValue(B,"_gl_DrawID",Jt),vt.render(Zt[Jt]/ft,Be[Jt])}else if(J.isInstancedMesh)vt.renderInstances(Oe,Pt,J.count);else if(Q.isInstancedBufferGeometry){let Zt=Q._maxInstanceCount!==void 0?Q._maxInstanceCount:1/0,Be=Math.min(Q.instanceCount,Zt);vt.renderInstances(Oe,Pt,Be)}else vt.render(Oe,Pt)};function Hh(T,z,Q,Y){D!==null&&T.isNodeMaterial&&D.setObject(Y,T),j===!0&&re.setState(T,Q,!1),T.transparent===!0&&T.side===Gn&&T.forceSinglePass===!1?(T.side=Xt,T.needsUpdate=!0,tr(T,z,Y),T.side=Ri,T.needsUpdate=!0,tr(T,z,Y),T.side=Gn):tr(T,z,Y)}this.compile=function(T,z,Q=null){Q===null&&(Q=T),D!==null&&D.renderStart(T,z,Q),w=ne.get(Q),w.init(z),M.push(w),Q.traverseVisible(function(J){J.isLight&&J.layers.test(z.layers)&&(w.pushLight(J),J.castShadow&&w.pushShadow(J))}),T!==Q&&T.traverseVisible(function(J){J.isLight&&J.layers.test(z.layers)&&(w.pushLight(J),J.castShadow&&w.pushShadow(J))}),w.setupLights(),D!==null&&D.updateLights(w.state.lightsArray),ge=this.localClippingEnabled,j=re.init(this.clippingPlanes,ge),j===!0&&re.setGlobalState(this.clippingPlanes,z),D!==null&&be.render(w.state.shadowsArray,Q,z);let Y=new Set;return T.traverse(function(J){if(!(J.isMesh||J.isPoints||J.isLine||J.isSprite))return;let Ee=J.material;if(Ee)if(Array.isArray(Ee))for(let Fe=0;Fe<Ee.length;Fe++){let Ae=Ee[Fe];Hh(Ae,Q,z,J),Y.add(Ae)}else Hh(Ee,Q,z,J),Y.add(Ee)}),w=M.pop(),D!==null&&D.renderEnd(),Y},this.compileAsync=function(T,z,Q=null){let Y=this.compile(T,z,Q);return new Promise(J=>{function Ee(){if(Y.forEach(function(Fe){let Ne=He.get(Fe).currentProgram;(Ne===void 0||Ne.isReady())&&Y.delete(Fe)}),Y.size===0){J(T);return}setTimeout(Ee,10)}je.get("KHR_parallel_shader_compile")!==null?Ee():setTimeout(Ee,10)})};let Dl=null;function cp(T){Dl&&Dl(T)}function Wh(){Ni.stop()}function Xh(){Ni.start()}let Ni=new id;Ni.setAnimationLoop(cp),typeof self<"u"&&Ni.setContext(self),this.setAnimationLoop=function(T){Dl=T,_e.setAnimationLoop(T),T===null?Ni.stop():Ni.start()},_e.addEventListener("sessionstart",Wh),_e.addEventListener("sessionend",Xh),this.render=function(T,z){if(z!==void 0&&z.isCamera!==!0){Ye("WebGLRenderer.render: camera is not an instance of THREE.Camera.");return}if(R===!0)return;D!==null&&D.renderStart(T,z);let Q=_e.enabled===!0&&_e.isPresenting===!0,Y=C!==null&&(ee===null||Q)&&C.begin(E,ee);if(T.matrixWorldAutoUpdate===!0&&T.updateMatrixWorld(),z.parent===null&&z.matrixWorldAutoUpdate===!0&&z.updateMatrixWorld(),_e.enabled===!0&&_e.isPresenting===!0&&(C===null||C.isCompositing()===!1)&&(_e.cameraAutoUpdate===!0&&_e.updateCamera(z),z=_e.getCamera()),T.isScene===!0&&T.onBeforeRender(E,T,z,ee),w=ne.get(T,M.length),w.init(z),w.state.textureUnits=qe.getTextureUnits(),M.push(w),le.multiplyMatrices(z.projectionMatrix,z.matrixWorldInverse),z.isArrayCamera?X.setFromArrayCamera(z):N.setFromProjectionMatrix(le,xn,z.reversedDepth),ge=this.localClippingEnabled,j=re.init(this.clippingPlanes,ge),b=Te.get(T,y.length),b.init(),y.push(b),this.info.render.frame++,_e.enabled===!0&&_e.isPresenting===!0){let Fe=E.xr.getDepthSensingMesh();Fe!==null&&Ll(Fe,z,-1/0,E.sortObjects)}Ll(T,z,0,E.sortObjects),b.finish(),D!==null&&D.updateLights(w.state.lightsArray),E.sortObjects===!0&&b.sort(H,se),Je=_e.enabled===!1||_e.isPresenting===!1||_e.hasDepthSensing()===!1,Je&&F.addToRenderList(b,T),this.info.autoReset===!0&&this.info.reset(),j===!0&&re.beginShadows();let J=w.state.shadowsArray;if(be.render(J,T,z),j===!0&&re.endShadows(),(Y&&C.hasRenderPass())===!1){let Fe=b.opaque,Ae=b.transmissive;if(w.setupLights(),z.isArrayCamera){let Ne=z.cameras;if(Ae.length>0)for(let Ve=0,$e=Ne.length;Ve<$e;Ve++){let ot=Ne[Ve];Yh(Fe,Ae,T,ot)}Je&&F.render(T);for(let Ve=0,$e=Ne.length;Ve<$e;Ve++){let ot=Ne[Ve];qh(b,T,ot,ot.viewport)}}else Ae.length>0&&Yh(Fe,Ae,T,z),Je&&F.render(T),qh(b,T,z)}ee!==null&&U===0&&(qe.updateMultisampleRenderTarget(ee),qe.updateRenderTargetMipmap(ee)),Y&&C.end(E),T.isScene===!0&&T.onAfterRender(E,T,z),Se.resetDefaultState(),W=-1,K=null,M.pop(),M.length>0?(w=M[M.length-1],qe.setTextureUnits(w.state.textureUnits),j===!0&&re.setGlobalState(E.clippingPlanes,w.state.camera)):w=null,y.pop(),y.length>0?b=y[y.length-1]:b=null,D!==null&&D.renderEnd()};function Ll(T,z,Q,Y){if(T.visible===!1)return;if(T.layers.test(z.layers)){if(T.isGroup)Q=T.renderOrder;else if(T.isLOD)T.autoUpdate===!0&&T.update(z);else if(T.isLight)w.pushLight(T),T.castShadow&&w.pushShadow(T);else if(T.isSprite){let Fe=z.isArrayCamera?X:N;if(!T.frustumCulled||T.intersectsFrustum(Fe)){Y&&xe.setFromMatrixPosition(T.matrixWorld).applyMatrix4(le);let Ae=me.update(T),Ne=T.material;Ne.visible&&b.push(T,Ae,Ne,Q,xe.z,null,z)}}else if(T.isMesh||T.isLine||T.isPoints){let Fe=z.isArrayCamera?X:N;if(!T.frustumCulled||T.intersectsFrustum(Fe)){let Ae=me.update(T),Ne=T.material;if(Y&&(T.boundingSphere!==void 0?(T.boundingSphere===null&&T.computeBoundingSphere(),xe.copy(T.boundingSphere.center)):(Ae.boundingSphere===null&&Ae.computeBoundingSphere(),xe.copy(Ae.boundingSphere.center)),xe.applyMatrix4(T.matrixWorld).applyMatrix4(le)),Array.isArray(Ne)){let Ve=Ae.groups;for(let $e=0,ot=Ve.length;$e<ot;$e++){let Oe=Ve[$e],at=Ne[Oe.materialIndex];at&&at.visible&&b.push(T,Ae,at,Q,xe.z,Oe,z)}}else Ne.visible&&b.push(T,Ae,Ne,Q,xe.z,null,z)}}}let Ee=T.children;for(let Fe=0,Ae=Ee.length;Fe<Ae;Fe++)Ll(Ee[Fe],z,Q,Y)}function qh(T,z,Q,Y){let{opaque:J,transmissive:Ee,transparent:Fe}=T;w.setupLightsView(Q),j===!0&&re.setGlobalState(E.clippingPlanes,Q),Y&&Me.viewport($.copy(Y)),J.length>0&&_o(J,z,Q),Ee.length>0&&_o(Ee,z,Q),Fe.length>0&&_o(Fe,z,Q),Me.buffers.depth.setTest(!0),Me.buffers.depth.setMask(!0),Me.buffers.color.setMask(!0),Me.setPolygonOffset(!1)}function Yh(T,z,Q,Y){if((Q.isScene===!0?Q.overrideMaterial:null)!==null)return;if(w.state.transmissionRenderTarget[Y.id]===void 0){let Oe=je.has("EXT_color_buffer_half_float")||je.has("EXT_color_buffer_float");w.state.transmissionRenderTarget[Y.id]=new hn(1,1,{generateMipmaps:!0,type:Oe?In:on,minFilter:fi,samples:Math.max(4,rt.samples),stencilBuffer:r,resolveDepthBuffer:!1,resolveStencilBuffer:!1,storeMultisampledDepthBuffer:!1,storeMultisampledStencilBuffer:!1,colorSpace:lt.workingColorSpace})}let Ee=w.state.transmissionRenderTarget[Y.id],Fe=Y.viewport||$;Ee.setSize(Fe.z*E.transmissionResolutionScale,Fe.w*E.transmissionResolutionScale);let Ae=E.getRenderTarget(),Ne=E.getActiveCubeFace(),Ve=E.getActiveMipmapLevel();E.setRenderTarget(Ee),E.getClearColor(q),ie=E.getClearAlpha(),ie<1&&E.setClearColor(16777215,.5),E.clear(),Je&&F.render(Q);let $e=E.toneMapping;E.toneMapping=Hn;let ot=Y.viewport;if(Y.viewport!==void 0&&(Y.viewport=void 0),w.setupLightsView(Y),j===!0&&re.setGlobalState(E.clippingPlanes,Y),_o(T,Q,Y),qe.updateMultisampleRenderTarget(Ee),qe.updateRenderTargetMipmap(Ee),je.has("WEBGL_multisampled_render_to_texture")===!1){let Oe=!1;for(let at=0,Pt=z.length;at<Pt;at++){let St=z[at],{object:vt,geometry:Zt,material:Be,group:en}=St;if(Be.side===Gn&&vt.layers.test(Y.layers)){let ft=Be.side;Be.side=Xt,Be.needsUpdate=!0,Zh(vt,Q,Y,Zt,Be,en),Be.side=ft,Be.needsUpdate=!0,Oe=!0}}Oe===!0&&(qe.updateMultisampleRenderTarget(Ee),qe.updateRenderTargetMipmap(Ee))}E.setRenderTarget(Ae,Ne,Ve),E.setClearColor(q,ie),ot!==void 0&&(Y.viewport=ot),E.toneMapping=$e}function _o(T,z,Q){let Y=z.isScene===!0?z.overrideMaterial:null;for(let J=0,Ee=T.length;J<Ee;J++){let Fe=T[J],{object:Ae,geometry:Ne,group:Ve}=Fe,$e=Fe.material;$e.allowOverride===!0&&Y!==null&&($e=Y),Ae.layers.test(Q.layers)&&Zh(Ae,z,Q,Ne,$e,Ve)}}function Zh(T,z,Q,Y,J,Ee){D!==null&&J.isNodeMaterial&&D.setObject(T,J),T.onBeforeRender(E,z,Q,Y,J,Ee),T.modelViewMatrix.multiplyMatrices(Q.matrixWorldInverse,T.matrixWorld),T.normalMatrix.getNormalMatrix(T.modelViewMatrix),J.onBeforeRender(E,z,Q,Y,T,Ee),J.transparent===!0&&J.side===Gn&&J.forceSinglePass===!1?(J.side=Xt,J.needsUpdate=!0,E.renderBufferDirect(Q,z,Y,J,T,Ee),J.side=Ri,J.needsUpdate=!0,E.renderBufferDirect(Q,z,Y,J,T,Ee),J.side=Gn):E.renderBufferDirect(Q,z,Y,J,T,Ee),T.onAfterRender(E,z,Q,Y,J,Ee)}function Jh(T){let z=0;for(let Q of T)Q.texture!==null&&z++;return z}function tr(T,z,Q){z.isScene!==!0&&(z=Ue);let Y=He.get(T),J=w.state.lights,Ee=w.state.shadowsArray,Fe=J.state.version,Ae=ae.getParameters(T,J.state,Ee,z,Q,w.state.lightProbeGridArray),Ne=ae.getProgramCacheKey(Ae),Ve=Y.programs;Y.environment=T.isMeshStandardMaterial||T.isMeshLambertMaterial||T.isMeshPhongMaterial?z.environment:null,Y.fog=z.fog;let $e=T.isMeshStandardMaterial||T.isMeshLambertMaterial&&!T.envMap||T.isMeshPhongMaterial&&!T.envMap;Y.envMap=I.get(T.envMap||Y.environment,$e),Y.envMapRotation=Y.environment!==null&&T.envMap===null?z.environmentRotation:T.envMapRotation,Ve===void 0&&(T.addEventListener("dispose",mt),Ve=new Map,Y.programs=Ve,Y.programVariants=new Map);let ot=Ve.get(Ne);if(ot!==void 0){if(Y.currentProgram===ot&&Y.lightsStateVersion===Fe)return Kh(T,Ae),ot}else Ae.uniforms=ae.getUniforms(T),D!==null&&T.isNodeMaterial&&D.build(T,Q,Ae),T.onBeforeCompile(Ae,E),ot=ae.acquireProgram(Ae,Ne),Ve.set(Ne,ot),Y.uniforms=Ae.uniforms;let Oe=Y.uniforms;return(!T.isShaderMaterial&&!T.isRawShaderMaterial||T.clipping===!0)&&(Oe.clippingPlanes=re.uniform),Kh(T,Ae),Y.needsLights=dp(T),Y.lightsStateVersion=Fe,Y.needsLights&&(Oe.ambientLightColor.value=J.state.ambient,Oe.lightProbe.value=J.state.probe,Oe.sunLights.value=J.state.sun,Oe.sunLightShadows.value=J.state.sunShadow,Oe.directionalLights.value=J.state.directional,Oe.directionalLightShadows.value=J.state.directionalShadow,Oe.spotLights.value=J.state.spot,Oe.spotLightShadows.value=J.state.spotShadow,Oe.rectAreaLights.value=J.state.rectArea,Oe.ltc_1.value=J.state.rectAreaLTC1,Oe.ltc_2.value=J.state.rectAreaLTC2,Oe.pointLights.value=J.state.point,Oe.pointLightShadows.value=J.state.pointShadow,Oe.hemisphereLights.value=J.state.hemi,Oe.sunShadowMatrix.value=J.state.sunShadowMatrix,Oe.sunShadowCascade.value=J.state.sunShadowCascade,Oe.directionalShadowMatrix.value=J.state.directionalShadowMatrix,Oe.spotLightMatrix.value=J.state.spotLightMatrix,Oe.spotLightMap.value=J.state.spotLightMap,Oe.pointShadowMatrix.value=J.state.pointShadowMatrix),Y.numLightProbeGrids=Jh(w.state.lightProbeGridArray),Y.currentProgram=ot,Y.uniformsList=null,ot}function hp(T,z,Q,Y,J){let Ee=He.get(T),Fe=Ee.currentProgram.id+","+Y+","+J,Ae=Ee.programVariants.get(Fe);return Ae===void 0?(Ae=tr(T,z,Q),Ee.programVariants.set(Fe,Ae)):(Ee.currentProgram=Ae,Ee.uniformsList=null,Ee.outputColorSpace=Y,Ee.toneMapping=J),Ae}function $h(T){if(T.uniformsList===null){let z=T.currentProgram.getUniforms();T.uniformsList=qs.seqWithValue(z.seq,T.uniforms)}return T.uniformsList}function Kh(T,z){let Q=He.get(T);Q.outputColorSpace=z.outputColorSpace,Q.batching=z.batching,Q.batchingColor=z.batchingColor,Q.instancing=z.instancing,Q.instancingColor=z.instancingColor,Q.instancingMorph=z.instancingMorph,Q.skinning=z.skinning,Q.morphTargets=z.morphTargets,Q.morphNormals=z.morphNormals,Q.morphColors=z.morphColors,Q.morphTargetsCount=z.morphTargetsCount,Q.numClippingPlanes=z.numClippingPlanes,Q.numIntersection=z.numClipIntersection,Q.vertexAlphas=z.vertexAlphas,Q.vertexTangents=z.vertexTangents,Q.toneMapping=z.toneMapping}function up(T,z,Q,Y,J){z.isScene!==!0&&(z=Ue),qe.resetTextureUnits();let Ee=z.fog,Fe=Y.isMeshStandardMaterial||Y.isMeshLambertMaterial||Y.isMeshPhongMaterial?z.environment:null,Ae=ee===null?E.outputColorSpace:ee.isXRRenderTarget===!0?ee.texture.colorSpace:lt.workingColorSpace,Ne=Y.isMeshStandardMaterial||Y.isMeshLambertMaterial&&!Y.envMap||Y.isMeshPhongMaterial&&!Y.envMap,Ve=I.get(Y.envMap||Fe,Ne),$e=Y.vertexColors===!0&&!!Q.attributes.color&&Q.attributes.color.itemSize===4,ot=!!Q.attributes.tangent&&(!!Y.normalMap||Y.anisotropy>0),Oe=!!Q.morphAttributes.position,at=!!Q.morphAttributes.normal,Pt=!!Q.morphAttributes.color,St=Hn;Y.toneMapped&&(ee===null||ee.isXRRenderTarget===!0)&&(St=E.toneMapping);let vt=Q.morphAttributes.position||Q.morphAttributes.normal||Q.morphAttributes.color,Zt=vt!==void 0?vt.length:0,Be=He.get(Y),en=w.state.lights;if(j===!0&&(ge===!0||T!==K)){let Tt=T===K&&Y.id===W;re.setState(Y,T,Tt)}let ft=!1,Ui=!1;Y.version===Be.__version?Be.needsLights&&Be.lightsStateVersion!==en.state.version||J.isBatchedMesh&&Be.batching===!1||!J.isBatchedMesh&&Be.batching===!0||J.isBatchedMesh&&Be.batchingColor===!0&&J._colorsTexture===null||J.isBatchedMesh&&Be.batchingColor===!1&&J._colorsTexture!==null||J.isInstancedMesh&&Be.instancing===!1||!J.isInstancedMesh&&Be.instancing===!0||J.isSkinnedMesh&&Be.skinning===!1||!J.isSkinnedMesh&&Be.skinning===!0||J.isInstancedMesh&&Be.instancingColor===!0&&J.instanceColor===null||J.isInstancedMesh&&Be.instancingColor===!1&&J.instanceColor!==null||J.isInstancedMesh&&Be.instancingMorph===!0&&J.morphTexture===null||J.isInstancedMesh&&Be.instancingMorph===!1&&J.morphTexture!==null||Be.envMap!==Ve||Y.fog===!0&&Be.fog!==Ee||Be.numClippingPlanes!==void 0&&(Be.numClippingPlanes!==re.numPlanes||Be.numIntersection!==re.numIntersection)||Be.vertexAlphas!==$e||Be.vertexTangents!==ot||Be.morphTargets!==Oe||Be.morphNormals!==at||Be.morphColors!==Pt||Be.morphTargetsCount!==Zt||Be.numLightProbeGrids!==Jh(w.state.lightProbeGridArray)?ft=!0:(Be.outputColorSpace!==Ae||Be.toneMapping!==St)&&(Ui=!0):(ft=!0,Be.__version=Y.version);let Jt=Be.currentProgram;(ft===!0||Ui===!0)&&(Jt=Ui===!0?hp(Y,z,J,Ae,St):tr(Y,z,J),D&&Y.isNodeMaterial&&D.onUpdateProgram(Y,Jt,Be));let xo=!1,_i=!1,Fi=!1,ht=Jt.getUniforms(),Ht=Be.uniforms;if(Me.useProgram(Jt.program)&&(xo=!0,_i=!0,Fi=!0),Y.id!==W&&(W=Y.id,_i=!0),xo||K!==T){Me.buffers.depth.getReversed()&&T.reversedDepth!==!0&&(T._reversedDepth=!0,T.updateProjectionMatrix()),ht.setValue(B,"projectionMatrix",T.projectionMatrix),ht.setValue(B,"viewMatrix",T.matrixWorldInverse);let zt=ht.map.cameraPosition;zt!==void 0&&zt.setValue(B,Re.setFromMatrixPosition(T.matrixWorld)),rt.logarithmicDepthBuffer&&ht.setValue(B,"logDepthBufFC",2/(Math.log(T.far+1)/Math.LN2)),(Y.isMeshPhongMaterial||Y.isMeshToonMaterial||Y.isMeshLambertMaterial||Y.isMeshBasicMaterial||Y.isMeshStandardMaterial||Y.isShaderMaterial)&&ht.setValue(B,"isOrthographic",T.isOrthographicCamera===!0),K!==T&&(K=T,_i=!0,Fi=!0)}if(Be.needsLights&&(en.state.sunShadowMap.length>0&&ht.setValue(B,"sunShadowMap",en.state.sunShadowMap,qe),en.state.directionalShadowMap.length>0&&ht.setValue(B,"directionalShadowMap",en.state.directionalShadowMap,qe),en.state.spotShadowMap.length>0&&ht.setValue(B,"spotShadowMap",en.state.spotShadowMap,qe),en.state.pointShadowMap.length>0&&ht.setValue(B,"pointShadowMap",en.state.pointShadowMap,qe)),J.isSkinnedMesh){ht.setOptional(B,J,"bindMatrix"),ht.setOptional(B,J,"bindMatrixInverse");let Tt=J.skeleton;Tt&&(Tt.boneTexture===null&&Tt.computeBoneTexture(),ht.setValue(B,"boneTexture",Tt.boneTexture,qe))}J.isBatchedMesh&&(ht.setOptional(B,J,"batchingTexture"),ht.setValue(B,"batchingTexture",J._matricesTexture,qe),ht.setOptional(B,J,"batchingIdTexture"),ht.setValue(B,"batchingIdTexture",J._indirectTexture,qe),ht.setOptional(B,J,"batchingColorTexture"),J._colorsTexture!==null&&ht.setValue(B,"batchingColorTexture",J._colorsTexture,qe));let wn=Q.morphAttributes;if((wn.position!==void 0||wn.normal!==void 0||wn.color!==void 0)&&oe.update(J,Q,Jt),(_i||Be.receiveShadow!==J.receiveShadow)&&(Be.receiveShadow=J.receiveShadow,ht.setValue(B,"receiveShadow",J.receiveShadow)),(Y.isMeshStandardMaterial||Y.isMeshLambertMaterial||Y.isMeshPhongMaterial)&&Y.envMap===null&&z.environment!==null&&(Ht.envMapIntensity.value=z.environmentIntensity),Ht.dfgLUT!==void 0&&(Ht.dfgLUT.value=My()),_i){if(ht.setValue(B,"toneMappingExposure",E.toneMappingExposure),Be.needsLights&&fp(Ht,Fi),Ee&&Y.fog===!0&&ve.refreshFogUniforms(Ht,Ee),ve.refreshMaterialUniforms(Ht,Y,ue,Ce,w.state.transmissionRenderTarget[T.id]),Be.needsLights&&Be.numLightProbeGrids>0){let Tt=x,zt=0;for(let Jn of w.state.lightProbeGridArray)Jn.texture!==null&&(Tt.textures[zt]=Jn.texture,Tt.min[zt]=Jn.boundingBox.min,Tt.max[zt]=Jn.boundingBox.max,Tt.resolution[zt]=Jn.resolution,zt++);for(let Jn in Tt)Tt[Jn].length=zt;Ht.probesSH.value=Tt.textures,Ht.probesMin.value=Tt.min,Ht.probesMax.value=Tt.max,Ht.probesResolution.value=Tt.resolution}qs.upload(B,$h(Be),Ht,qe)}if(Y.isShaderMaterial&&Y.uniformsNeedUpdate===!0&&(qs.upload(B,$h(Be),Ht,qe),Y.uniformsNeedUpdate=!1),Y.isSpriteMaterial&&ht.setValue(B,"center",J.center),ht.setValue(B,"modelViewMatrix",J.modelViewMatrix),ht.setValue(B,"normalMatrix",J.normalMatrix),ht.setValue(B,"modelMatrix",J.matrixWorld),Y.uniformsGroups!==void 0){let Tt=Y.uniformsGroups;for(let zt=0,Jn=Tt.length;zt<Jn;zt++){let Qh=Tt[zt];Ge.update(Qh,Jt),Ge.bind(Qh,Jt)}}return Jt}function fp(T,z){T.ambientLightColor.needsUpdate=z,T.lightProbe.needsUpdate=z,T.sunLights.needsUpdate=z,T.sunLightShadows.needsUpdate=z,T.directionalLights.needsUpdate=z,T.directionalLightShadows.needsUpdate=z,T.pointLights.needsUpdate=z,T.pointLightShadows.needsUpdate=z,T.spotLights.needsUpdate=z,T.spotLightShadows.needsUpdate=z,T.rectAreaLights.needsUpdate=z,T.hemisphereLights.needsUpdate=z}function dp(T){return T.isMeshLambertMaterial||T.isMeshToonMaterial||T.isMeshPhongMaterial||T.isMeshStandardMaterial||T.isShadowMaterial||T.isShaderMaterial&&T.lights===!0}this.getActiveCubeFace=function(){return G},this.getActiveMipmapLevel=function(){return U},this.getRenderTarget=function(){return ee},this.setRenderTargetTextures=function(T,z,Q){let Y=He.get(T);Y.__autoAllocateDepthBuffer=T.resolveDepthBuffer===!1,Y.__autoAllocateDepthBuffer===!1&&(Y.__useRenderToTexture=!1),He.get(T.texture).__webglTexture=z,He.get(T.depthTexture).__webglTexture=Y.__autoAllocateDepthBuffer?void 0:Q,Y.__hasExternalTextures=!0},this.setRenderTargetFramebuffer=function(T,z){let Q=He.get(T);Q.__webglFramebuffer=z,Q.__useDefaultFramebuffer=z===void 0},this.setRenderTarget=function(T,z=0,Q=0){ee=T,G=z,U=Q;let Y=null,J=!1,Ee=!1;if(T){let Ae=He.get(T);if(Ae.__useDefaultFramebuffer!==void 0){Me.bindFramebuffer(B.FRAMEBUFFER,Ae.__webglFramebuffer),$.copy(T.viewport),he.copy(T.scissor),de=T.scissorTest,Me.viewport($),Me.scissor(he),Me.setScissorTest(de),W=-1;return}else if(Ae.__webglFramebuffer===void 0)qe.setupRenderTarget(T);else if(Ae.__hasExternalTextures)qe.rebindTextures(T,He.get(T.texture).__webglTexture,He.get(T.depthTexture).__webglTexture);else if(T.depthBuffer){let $e=T.depthTexture;if(Ae.__boundDepthTexture!==$e){if($e!==null&&He.has($e)&&(T.width!==$e.image.width||T.height!==$e.image.height))throw new Error("THREE.WebGLRenderer: Attached DepthTexture is initialized to the incorrect size.");qe.setupDepthRenderbuffer(T)}}let Ne=T.texture;(Ne.isData3DTexture||Ne.isDataArrayTexture||Ne.isCompressedArrayTexture)&&(Ee=!0);let Ve=He.get(T).__webglFramebuffer;T.isWebGLCubeRenderTarget?(Array.isArray(Ve[z])?Y=Ve[z][Q]:Y=Ve[z],J=!0):T.samples>0&&qe.useMultisampledRTT(T)===!1?Y=He.get(T).__webglMultisampledFramebuffer:Array.isArray(Ve)?Y=Ve[Q]:Y=Ve,$.copy(T.viewport),he.copy(T.scissor),de=T.scissorTest}else $.copy(fe).multiplyScalar(ue).floor(),he.copy(Le).multiplyScalar(ue).floor(),de=V;if(Q!==0&&(Y=k),Me.bindFramebuffer(B.FRAMEBUFFER,Y)&&Me.drawBuffers(T,Y),Me.viewport($),Me.scissor(he),Me.setScissorTest(de),J){let Ae=He.get(T.texture);B.framebufferTexture2D(B.FRAMEBUFFER,B.COLOR_ATTACHMENT0,B.TEXTURE_CUBE_MAP_POSITIVE_X+z,Ae.__webglTexture,Q)}else if(Ee){let Ae=z;for(let Ne=0;Ne<T.textures.length;Ne++){let Ve=He.get(T.textures[Ne]);B.framebufferTextureLayer(B.FRAMEBUFFER,B.COLOR_ATTACHMENT0+Ne,Ve.__webglTexture,Q,Ae)}}else if(T!==null&&Q!==0){let Ae=He.get(T.texture);B.framebufferTexture2D(B.FRAMEBUFFER,B.COLOR_ATTACHMENT0,B.TEXTURE_2D,Ae.__webglTexture,Q)}W=-1};function jh(T){let z=He.get(T);return(z.__readFormat!==T.format||z.__readType!==T.type)&&(z.__readFormat=T.format,z.__readType=T.type,z.__formatReadable=rt.textureFormatReadable(T.format),z.__typeReadable=rt.textureTypeReadable(T.type)),z}this.readRenderTargetPixels=function(T,z,Q,Y,J,Ee,Fe,Ae=0){if(!(T&&T.isWebGLRenderTarget)){Ye("WebGLRenderer.readRenderTargetPixels: renderTarget is not THREE.WebGLRenderTarget.");return}let Ne=He.get(T).__webglFramebuffer;if(T.isWebGLCubeRenderTarget&&Fe!==void 0&&(Ne=Ne[Fe],Array.isArray(Ne)&&(Ne=Ne[0])),Ne){Me.bindFramebuffer(B.FRAMEBUFFER,Ne);try{let Ve=T.textures[Ae],$e=Ve.format,ot=Ve.type;T.textures.length>1&&B.readBuffer(B.COLOR_ATTACHMENT0+Ae);let Oe=jh(Ve);if(Oe.__formatReadable===!1){Ye("WebGLRenderer.readRenderTargetPixels: renderTarget is not in RGBA or implementation defined format.");return}if(Oe.__typeReadable===!1){Ye("WebGLRenderer.readRenderTargetPixels: renderTarget is not in UnsignedByteType or implementation defined type.");return}z>=0&&z<=T.width-Y&&Q>=0&&Q<=T.height-J&&B.readPixels(z,Q,Y,J,ye.convert($e),ye.convert(ot),Ee)}finally{let Ve=ee!==null?He.get(ee).__webglFramebuffer:null;Me.bindFramebuffer(B.FRAMEBUFFER,Ve)}}},this.readRenderTargetPixelsAsync=async function(T,z,Q,Y,J,Ee,Fe,Ae=0){if(!(T&&T.isWebGLRenderTarget))throw new Error("THREE.WebGLRenderer.readRenderTargetPixels: renderTarget is not THREE.WebGLRenderTarget.");let Ne=He.get(T).__webglFramebuffer;if(T.isWebGLCubeRenderTarget&&Fe!==void 0&&(Ne=Ne[Fe],Array.isArray(Ne)&&(Ne=Ne[0])),Ne)if(z>=0&&z<=T.width-Y&&Q>=0&&Q<=T.height-J){Me.bindFramebuffer(B.FRAMEBUFFER,Ne);let Ve=T.textures[Ae],$e=Ve.format,ot=Ve.type;T.textures.length>1&&B.readBuffer(B.COLOR_ATTACHMENT0+Ae);let Oe=jh(Ve);if(Oe.__formatReadable===!1)throw new Error("THREE.WebGLRenderer.readRenderTargetPixelsAsync: renderTarget is not in RGBA or implementation defined format.");if(Oe.__typeReadable===!1)throw new Error("THREE.WebGLRenderer.readRenderTargetPixelsAsync: renderTarget is not in UnsignedByteType or implementation defined type.");let at=B.createBuffer();B.bindBuffer(B.PIXEL_PACK_BUFFER,at),B.bufferData(B.PIXEL_PACK_BUFFER,Ee.byteLength,B.STREAM_READ),B.readPixels(z,Q,Y,J,ye.convert($e),ye.convert(ot),0),B.bindBuffer(B.PIXEL_PACK_BUFFER,null);let Pt=ee!==null?He.get(ee).__webglFramebuffer:null;Me.bindFramebuffer(B.FRAMEBUFFER,Pt);let St=B.fenceSync(B.SYNC_GPU_COMMANDS_COMPLETE,0);return B.flush(),await Af(B,St,4),B.bindBuffer(B.PIXEL_PACK_BUFFER,at),B.getBufferSubData(B.PIXEL_PACK_BUFFER,0,Ee),B.bindBuffer(B.PIXEL_PACK_BUFFER,null),B.deleteBuffer(at),B.deleteSync(St),Ee}else throw new Error("THREE.WebGLRenderer.readRenderTargetPixelsAsync: requested read bounds are out of range.")},this.copyFramebufferToTexture=function(T,z=null,Q=0){let Y=Math.pow(2,-Q),J=Math.floor(T.image.width*Y),Ee=Math.floor(T.image.height*Y),Fe=z!==null?z.x:0,Ae=z!==null?z.y:0;qe.setTexture2D(T,0),B.copyTexSubImage2D(B.TEXTURE_2D,Q,0,0,Fe,Ae,J,Ee),Me.unbindTexture()},this.copyTextureToTexture=function(T,z,Q=null,Y=null,J=0,Ee=0){let Fe,Ae,Ne,Ve,$e,ot,Oe,at,Pt,St=T.isCompressedTexture?T.mipmaps[Ee]:T.image;if(Q!==null)Fe=Q.max.x-Q.min.x,Ae=Q.max.y-Q.min.y,Ne=Q.isBox3?Q.max.z-Q.min.z:1,Ve=Q.min.x,$e=Q.min.y,ot=Q.isBox3?Q.min.z:0;else{let ht=Math.pow(2,-J);Fe=Math.floor(St.width*ht),Ae=Math.floor(St.height*ht),T.isDataArrayTexture?Ne=St.depth:T.isData3DTexture?Ne=Math.floor(St.depth*ht):Ne=1,Ve=0,$e=0,ot=0}Y!==null?(Oe=Y.x,at=Y.y,Pt=Y.z):(Oe=0,at=0,Pt=0);let vt=ye.convert(z.format),Zt=ye.convert(z.type),Be;z.isData3DTexture?(qe.setTexture3D(z,0),Be=B.TEXTURE_3D):z.isDataArrayTexture||z.isCompressedArrayTexture?(qe.setTexture2DArray(z,0),Be=B.TEXTURE_2D_ARRAY):(qe.setTexture2D(z,0),Be=B.TEXTURE_2D),Me.activeTexture(B.TEXTURE0),Me.pixelStorei(B.UNPACK_FLIP_Y_WEBGL,z.flipY),Me.pixelStorei(B.UNPACK_PREMULTIPLY_ALPHA_WEBGL,z.premultiplyAlpha),Me.pixelStorei(B.UNPACK_ALIGNMENT,z.unpackAlignment);let en=Me.getParameter(B.UNPACK_ROW_LENGTH),ft=Me.getParameter(B.UNPACK_IMAGE_HEIGHT),Ui=Me.getParameter(B.UNPACK_SKIP_PIXELS),Jt=Me.getParameter(B.UNPACK_SKIP_ROWS),xo=Me.getParameter(B.UNPACK_SKIP_IMAGES);Me.pixelStorei(B.UNPACK_ROW_LENGTH,St.width),Me.pixelStorei(B.UNPACK_IMAGE_HEIGHT,St.height),Me.pixelStorei(B.UNPACK_SKIP_PIXELS,Ve),Me.pixelStorei(B.UNPACK_SKIP_ROWS,$e),Me.pixelStorei(B.UNPACK_SKIP_IMAGES,ot);let _i=T.isDataArrayTexture||T.isData3DTexture,Fi=z.isDataArrayTexture||z.isData3DTexture;if(T.isDepthTexture){let ht=He.get(T),Ht=He.get(z),wn=He.get(ht.__renderTarget),Tt=He.get(Ht.__renderTarget);Me.bindFramebuffer(B.READ_FRAMEBUFFER,wn.__webglFramebuffer),Me.bindFramebuffer(B.DRAW_FRAMEBUFFER,Tt.__webglFramebuffer);for(let zt=0;zt<Ne;zt++)_i&&(B.framebufferTextureLayer(B.READ_FRAMEBUFFER,B.COLOR_ATTACHMENT0,He.get(T).__webglTexture,J,ot+zt),B.framebufferTextureLayer(B.DRAW_FRAMEBUFFER,B.COLOR_ATTACHMENT0,He.get(z).__webglTexture,Ee,Pt+zt)),B.blitFramebuffer(Ve,$e,Ve+Fe,$e+Ae,Oe,at,Oe+Fe,at+Ae,B.DEPTH_BUFFER_BIT,B.NEAREST);Me.bindFramebuffer(B.READ_FRAMEBUFFER,null),Me.bindFramebuffer(B.DRAW_FRAMEBUFFER,null)}else if(J!==0||T.isRenderTargetTexture||He.has(T)){let ht=He.get(T),Ht=He.get(z);Me.bindFramebuffer(B.READ_FRAMEBUFFER,P),Me.bindFramebuffer(B.DRAW_FRAMEBUFFER,O);for(let wn=0;wn<Ne;wn++)_i?B.framebufferTextureLayer(B.READ_FRAMEBUFFER,B.COLOR_ATTACHMENT0,ht.__webglTexture,J,ot+wn):B.framebufferTexture2D(B.READ_FRAMEBUFFER,B.COLOR_ATTACHMENT0,B.TEXTURE_2D,ht.__webglTexture,J),Fi?B.framebufferTextureLayer(B.DRAW_FRAMEBUFFER,B.COLOR_ATTACHMENT0,Ht.__webglTexture,Ee,Pt+wn):B.framebufferTexture2D(B.DRAW_FRAMEBUFFER,B.COLOR_ATTACHMENT0,B.TEXTURE_2D,Ht.__webglTexture,Ee),J!==0?B.blitFramebuffer(Ve,$e,Ve+Fe,$e+Ae,Oe,at,Oe+Fe,at+Ae,B.COLOR_BUFFER_BIT,B.NEAREST):Fi?B.copyTexSubImage3D(Be,Ee,Oe,at,Pt+wn,Ve,$e,Fe,Ae):B.copyTexSubImage2D(Be,Ee,Oe,at,Ve,$e,Fe,Ae);Me.bindFramebuffer(B.READ_FRAMEBUFFER,null),Me.bindFramebuffer(B.DRAW_FRAMEBUFFER,null)}else Fi?T.isDataTexture||T.isData3DTexture?B.texSubImage3D(Be,Ee,Oe,at,Pt,Fe,Ae,Ne,vt,Zt,St.data):z.isCompressedArrayTexture?B.compressedTexSubImage3D(Be,Ee,Oe,at,Pt,Fe,Ae,Ne,vt,St.data):B.texSubImage3D(Be,Ee,Oe,at,Pt,Fe,Ae,Ne,vt,Zt,St):T.isDataTexture?B.texSubImage2D(B.TEXTURE_2D,Ee,Oe,at,Fe,Ae,vt,Zt,St.data):T.isCompressedTexture?B.compressedTexSubImage2D(B.TEXTURE_2D,Ee,Oe,at,St.width,St.height,vt,St.data):B.texSubImage2D(B.TEXTURE_2D,Ee,Oe,at,Fe,Ae,vt,Zt,St);Me.pixelStorei(B.UNPACK_ROW_LENGTH,en),Me.pixelStorei(B.UNPACK_IMAGE_HEIGHT,ft),Me.pixelStorei(B.UNPACK_SKIP_PIXELS,Ui),Me.pixelStorei(B.UNPACK_SKIP_ROWS,Jt),Me.pixelStorei(B.UNPACK_SKIP_IMAGES,xo),Ee===0&&z.generateMipmaps===!0&&z.mipmapsAutoUpdate===!0&&B.generateMipmap(Be),Me.unbindTexture()},this.initRenderTarget=function(T){He.get(T).__webglFramebuffer===void 0&&qe.setupRenderTarget(T)},this.initTexture=function(T){T.isCubeTexture?qe.setTextureCube(T,0):T.isData3DTexture?qe.setTexture3D(T,0):T.isDataArrayTexture||T.isCompressedArrayTexture?qe.setTexture2DArray(T,0):qe.setTexture2D(T,0),Me.unbindTexture()},this.resetState=function(){G=0,U=0,ee=null,Me.reset(),Se.reset()},typeof __THREE_DEVTOOLS__<"u"&&__THREE_DEVTOOLS__.dispatchEvent(new CustomEvent("observe",{detail:this}))}get coordinateSystem(){return xn}get outputColorSpace(){return this._outputColorSpace}set outputColorSpace(e){this._outputColorSpace=e;let t=this.getContext();t.drawingBufferColorSpace=lt._getDrawingBufferColorSpace(e),t.unpackColorSpace=lt._getUnpackColorSpace()}};var ul=class extends si{constructor(){super(),this.name="RoomEnvironment",this.position.y=-3.5;let e=new sn;e.deleteAttribute("uv");let t=new fn({side:Xt}),n=new fn,i=new qi(16777215,900,28,2);i.position.set(.418,16.199,.3),this.add(i);let r=new ke(e,t);r.position.set(-.757,13.219,.717),r.scale.set(31.713,28.305,28.591),this.add(r);let o=new Sr(e,n,6),a=new At;a.position.set(-10.906,2.009,1.846),a.rotation.set(0,-.195,0),a.scale.set(2.328,7.905,4.651),a.updateMatrix(),o.setMatrixAt(0,a.matrix),a.position.set(-5.607,-.754,-.758),a.rotation.set(0,.994,0),a.scale.set(1.97,1.534,3.955),a.updateMatrix(),o.setMatrixAt(1,a.matrix),a.position.set(6.167,.857,7.803),a.rotation.set(0,.561,0),a.scale.set(3.927,6.285,3.687),a.updateMatrix(),o.setMatrixAt(2,a.matrix),a.position.set(-2.017,.018,6.124),a.rotation.set(0,.333,0),a.scale.set(2.002,4.566,2.064),a.updateMatrix(),o.setMatrixAt(3,a.matrix),a.position.set(2.291,-.756,-2.621),a.rotation.set(0,-.286,0),a.scale.set(1.546,1.552,1.496),a.updateMatrix(),o.setMatrixAt(4,a.matrix),a.position.set(-2.193,-.369,-5.547),a.rotation.set(0,.516,0),a.scale.set(3.875,3.487,2.986),a.updateMatrix(),o.setMatrixAt(5,a.matrix),this.add(o);let l=new ke(e,$s(50));l.position.set(-16.116,14.37,8.208),l.scale.set(.1,2.428,2.739),this.add(l);let c=new ke(e,$s(50));c.position.set(-16.109,18.021,-8.207),c.scale.set(.1,2.425,2.751),this.add(c);let u=new ke(e,$s(17));u.position.set(14.904,12.198,-1.832),u.scale.set(.15,4.265,6.331),this.add(u);let h=new ke(e,$s(43));h.position.set(-.462,8.89,14.52),h.scale.set(4.38,5.441,.088),this.add(h);let d=new ke(e,$s(20));d.position.set(3.235,11.486,-12.541),d.scale.set(2.5,2,.1),this.add(d);let f=new ke(e,$s(100));f.position.set(0,20,0),f.scale.set(1,.1,1),this.add(f)}dispose(){let e=new Set;this.traverse(t=>{t.isMesh&&(e.add(t.geometry),e.add(t.material))});for(let t of e)t.dispose()}};function $s(s){return new Fr({color:0,emissive:16777215,emissiveIntensity:s})}function pd(s,e,t){return s===1?{x:e,y:t}:{x:t,y:1-e}}function md(s,e){return s.width>0&&s.height>0&&Math.abs(s.width/s.height-e.width/e.height)<=1/s.height+1/e.height}function Ks(s){return s?`${s.screenId}:${s.width}:${s.height}:${s.orientation}`:""}function gd(s,e){let t=s.getObjectByName("left-half"),n=s.getObjectByName("right-half"),i=s.getObjectByName("cover-display"),r=s.getObjectByName("inner-display-left"),o=s.getObjectByName("inner-display-right");if(!t||!n||!(i instanceof ke)||!(r instanceof ke)||!(o instanceof ke))throw new Error("Duo model must have two hinge groups and all three display meshes");let a={1:[i],3:[r,o]},l=[1,3].map(v=>{let x=new Mt;for(let w of a[v])w.geometry.computeBoundingBox(),x.union(w.geometry.boundingBox);let b=x.getSize(new L);if(![b.x,b.y].every(w=>Number.isFinite(w)&&w>0))throw new Error("Invalid Duo display bounds");return{bounds:x,size:b}}),c=new ut,u=new ut;u.add(s),c.add(u);let h=Number.NaN,d=new Mt,f=new ze,m=new ze,_={1:new Wt({map:e[1],toneMapped:!1}),3:new Wt({map:e[3],toneMapped:!1})},p=new Map;for(let v of[1,3]){let{bounds:x,size:b}=l[v===1?0:1];for(let w of a[v]){let y=w.geometry.getAttribute("position"),M=new Float32Array(y.count*2);for(let C=0;C<y.count;C++){let E=(y.getX(C)-x.min.x)/b.x;M[C*2]=v===1?1-E:E,M[C*2+1]=(y.getY(C)-x.min.y)/b.y}w.geometry.setAttribute("uv",new gt(M,2)),p.set(w,w.material),w.material=_[v]}}let g=new ui,S=null;return{root:c,setAngle(v){if(v===h)return;h=v;let x=(180-Math.min(180,Math.max(0,v)))*Math.PI/360;t.rotation.y=x,n.rotation.y=-x,u.position.set(0,0,0),c.updateWorldMatrix(!0,!0),m.copy(c.matrixWorld).invert(),d.makeEmpty(),s.traverse(b=>{b instanceof ke&&(b.geometry.boundingBox||b.geometry.computeBoundingBox(),b.geometry.boundingBox&&(f.multiplyMatrices(m,b.matrixWorld),d.union(b.geometry.boundingBox.clone().applyMatrix4(f))))}),u.position.copy(d.getCenter(new L)).negate()},restFrames(v){c.updateWorldMatrix(!0,!0),m.copy(c.matrixWorld).invert();let x=(w,y)=>{let M=new Mt,C=new L,E=new L;for(let R of w)R.geometry.boundingBox||R.geometry.computeBoundingBox(),f.multiplyMatrices(m,R.matrixWorld),M.union(R.geometry.boundingBox.clone().applyMatrix4(f)),C.add(new L().fromBufferAttribute(R.geometry.getAttribute("normal"),0).transformDirection(f)),E.add(new L(0,1,0).transformDirection(f));return{face:y,center:M.getCenter(new L),normal:C.normalize(),up:E.normalize()}};if(v===1)return[x([i],"cover")];let b=x([r,o],"inside");return h>20&&h<165?[b,x([r],"left"),x([o],"right")]:[b]},leafRotation(v){return c.updateWorldMatrix(!0,!0),f.multiplyMatrices(c.matrixWorld.clone().invert(),(v==="left"?t:n).matrixWorld),new st().setFromRotationMatrix(f.extractRotation(f))},hingeLeafAt(v,x,b){if(!Number.isFinite(v)||!Number.isFinite(x))return null;c.updateMatrixWorld(!0),b.updateMatrixWorld(!0),g.setFromCamera(new pe(v*2-1,1-x*2),b);let w=g.intersectObject(c,!0)[0]?.object;for(;w;){if(w===t)return"left";if(w===n)return"right";w=w.parent??void 0}return null},cancelInput(){S=null},screenPoint(v,x,b,w,y,M=!1){let C=Ks(w);if(!w||!y||C!==y)return S=null,null;if(c.updateMatrixWorld(!0),b.updateMatrixWorld(!0),g.setFromCamera(new pe(v*2-1,1-x*2),b),M&&S?.key===C){let U=g.ray.intersectPlane(S.plane,new L),ee=U?S.triangle.getBarycoord(U,new L):null;if(!ee)return null;let W=S.uv[0].clone().multiplyScalar(ee.x).addScaledVector(S.uv[1],ee.y).addScaledVector(S.uv[2],ee.z);return pd(S.panel,W.x,1-W.y)}let E=g.intersectObject(c,!0)[0];if(!E?.uv||!E.face||!(E.object instanceof ke))return null;let R=a[1].includes(E.object)?1:a[3].includes(E.object)?3:null;if(!R||R!==w.screenId)return null;let D=[E.face.a,E.face.b,E.face.c],k=E.object.geometry.getAttribute("position"),P=new On(...D.map(U=>new L().fromBufferAttribute(k,U).applyMatrix4(E.object.matrixWorld))),O=E.object.geometry.getAttribute("uv"),G=D.map(U=>new pe().fromBufferAttribute(O,U));return S={panel:R,triangle:P,uv:G,plane:P.getPlane(new tn),key:C},pd(R,E.uv.x,1-E.uv.y)},dispose(){for(let[v,x]of p)v.material=x;for(let v of Object.values(_))v.map=null,v.dispose();u.remove(s),S=null}}}function mh(s,e=Math.PI/3){let t=s.index?s.toNonIndexed():s,n=t.attributes.position,i=n.count,r;if(n.isBufferAttribute===!0&&n.itemSize===3&&n.normalized===!1)r=n.array;else{r=new Float64Array(i*3);for(let x=0;x<i;x++)r[3*x+0]=n.getX(x),r[3*x+1]=n.getY(x),r[3*x+2]=n.getZ(x)}let o=Math.cos(e),a=(1+1e-10)*100,l=i/3,c=new Float64Array(l*3);for(let x=0;x<l;x++){let b=9*x,w=r[b+0],y=r[b+1],M=r[b+2],C=r[b+3],E=r[b+4],R=r[b+5],D=r[b+6],k=r[b+7],P=r[b+8],O=D-C,G=k-E,U=P-R,ee=w-C,W=y-E,K=M-R,$=G*K-U*W,he=U*ee-O*K,de=O*W-G*ee,q=1/(Math.sqrt($*$+he*he+de*de)||1);c[3*x+0]=$*q,c[3*x+1]=he*q,c[3*x+2]=de*q}let u=new Int32Array(i),h=new Float64Array(i*3),d=1;for(;d<i*2;)d<<=1;let f=d-1,m=new Int32Array(d),_=0;for(let x=0;x<i;x++){let b=3*x,w=Math.trunc(r[b+0]*a),y=Math.trunc(r[b+1]*a),M=Math.trunc(r[b+2]*a),C=(Math.imul(w,73856093)^Math.imul(y,19349663)^Math.imul(M,83492791))&f;for(;;){let E=m[C];if(E===0){let D=3*_;h[D+0]=w,h[D+1]=y,h[D+2]=M,m[C]=_+1,u[x]=_++;break}let R=3*(E-1);if(h[R+0]===w&&h[R+1]===y&&h[R+2]===M){u[x]=E-1;break}C=C+1&f}}let p=new Int32Array(_+1);for(let x=0;x<i;x++)p[u[x]+1]++;for(let x=0;x<_;x++)p[x+1]+=p[x];let g=new Int32Array(i),S=p.slice(0,_);for(let x=0;x<l;x++){let b=3*x;g[S[u[b+0]]++]=x,g[S[u[b+1]]++]=x,g[S[u[b+2]]++]=x}let v=new Float32Array(i*3);for(let x=0;x<l;x++){let b=3*x,w=c[b+0],y=c[b+1],M=c[b+2];for(let C=0;C<3;C++){let E=b+C,R=u[E],D=0,k=0,P=0;for(let G=p[R],U=p[R+1];G<U;G++){let ee=3*g[G],W=c[ee+0],K=c[ee+1],$=c[ee+2];w*W+y*K+M*$>o&&(D+=W,k+=K,P+=$)}let O=1/(Math.sqrt(D*D+k*k+P*P)||1);v[3*E+0]=D*O,v[3*E+1]=k*O,v[3*E+2]=P*O}}return t.setAttribute("normal",new gt(v,3,!1)),t}var Rt=Uint8Array,pn=Uint16Array,Th=Int32Array,fl=new Rt([0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2,3,3,3,3,4,4,4,4,5,5,5,5,0,0,0,0]),dl=new Rt([0,0,0,0,1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9,10,10,11,11,12,12,13,13,0,0]),vh=new Rt([16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15]),bd=function(s,e){for(var t=new pn(31),n=0;n<31;++n)t[n]=e+=1<<s[n-1];for(var i=new Th(t[30]),n=1;n<30;++n)for(var r=t[n];r<t[n+1];++r)i[r]=r-t[n]<<5|n;return{b:t,r:i}},wd=bd(fl,2),Ad=wd.b,Sh=wd.r;Ad[28]=258,Sh[258]=28;var Td=bd(dl,0),by=Td.b,_d=Td.r,Mh=new pn(32768);for(dt=0;dt<32768;++dt)pi=(dt&43690)>>1|(dt&21845)<<1,pi=(pi&52428)>>2|(pi&13107)<<2,pi=(pi&61680)>>4|(pi&3855)<<4,Mh[dt]=((pi&65280)>>8|(pi&255)<<8)>>1;var pi,dt,Yn=(function(s,e,t){for(var n=s.length,i=0,r=new pn(e);i<n;++i)s[i]&&++r[s[i]-1];var o=new pn(e);for(i=1;i<e;++i)o[i]=o[i-1]+r[i-1]<<1;var a;if(t){a=new pn(1<<e);var l=15-e;for(i=0;i<n;++i)if(s[i])for(var c=i<<4|s[i],u=e-s[i],h=o[s[i]-1]++<<u,d=h|(1<<u)-1;h<=d;++h)a[Mh[h]>>l]=c}else for(a=new pn(n),i=0;i<n;++i)s[i]&&(a[i]=Mh[o[s[i]-1]++]>>15-s[i]);return a}),Ii=new Rt(288);for(dt=0;dt<144;++dt)Ii[dt]=8;var dt;for(dt=144;dt<256;++dt)Ii[dt]=9;var dt;for(dt=256;dt<280;++dt)Ii[dt]=7;var dt;for(dt=280;dt<288;++dt)Ii[dt]=8;var dt,io=new Rt(32);for(dt=0;dt<32;++dt)io[dt]=5;var dt,wy=Yn(Ii,9,0),Ay=Yn(Ii,9,1),Ty=Yn(io,5,0),Ey=Yn(io,5,1),gh=function(s){for(var e=s[0],t=1;t<s.length;++t)s[t]>e&&(e=s[t]);return e},Dn=function(s,e,t){var n=e/8|0;return(s[n]|s[n+1]<<8)>>(e&7)&t},_h=function(s,e){var t=e/8|0;return(s[t]|s[t+1]<<8|s[t+2]<<16)>>(e&7)},Eh=function(s){return(s+7)/8|0},so=function(s,e,t){return(e==null||e<0)&&(e=0),(t==null||t>s.length)&&(t=s.length),new Rt(s.subarray(e,t))};var Ry=["unexpected EOF","invalid block type","invalid length/literal","invalid distance","stream finished","no stream handler",,"no callback","invalid UTF-8 data","extra field too long","date not in range 1980-2099","filename too long","stream finishing","invalid zip data"],Qt=function(s,e,t){var n=new Error(e||Ry[s]);if(n.code=s,Error.captureStackTrace&&Error.captureStackTrace(n,Qt),!t)throw n;return n},Cy=function(s,e,t,n){var i=s.length,r=n?n.length:0;if(!i||e.f&&!e.l)return t||new Rt(0);var o=!t,a=o||e.i!=2,l=e.i;o&&(t=new Rt(i*3));var c=function(fe){var Le=t.length;if(fe>Le){var V=new Rt(Math.max(Le*2,fe));V.set(t),t=V}},u=e.f||0,h=e.p||0,d=e.b||0,f=e.l,m=e.d,_=e.m,p=e.n,g=i*8;do{if(!f){u=Dn(s,h,1);var S=Dn(s,h+1,3);if(h+=3,S)if(S==1)f=Ay,m=Ey,_=9,p=5;else if(S==2){var w=Dn(s,h,31)+257,y=Dn(s,h+10,15)+4,M=w+Dn(s,h+5,31)+1;h+=14;for(var C=new Rt(M),E=new Rt(19),R=0;R<y;++R)E[vh[R]]=Dn(s,h+R*3,7);h+=y*3;for(var D=gh(E),k=(1<<D)-1,P=Yn(E,D,1),R=0;R<M;){var O=P[Dn(s,h,k)];h+=O&15;var v=O>>4;if(v<16)C[R++]=v;else{var G=0,U=0;for(v==16?(U=3+Dn(s,h,3),h+=2,G=C[R-1]):v==17?(U=3+Dn(s,h,7),h+=3):v==18&&(U=11+Dn(s,h,127),h+=7);U--;)C[R++]=G}}var ee=C.subarray(0,w),W=C.subarray(w);_=gh(ee),p=gh(W),f=Yn(ee,_,1),m=Yn(W,p,1)}else Qt(1);else{var v=Eh(h)+4,x=s[v-4]|s[v-3]<<8,b=v+x;if(b>i){l&&Qt(0);break}a&&c(d+x),t.set(s.subarray(v,b),d),e.b=d+=x,e.p=h=b*8,e.f=u;continue}if(h>g){l&&Qt(0);break}}a&&c(d+131072);for(var K=(1<<_)-1,$=(1<<p)-1,he=h;;he=h){var G=f[_h(s,h)&K],de=G>>4;if(h+=G&15,h>g){l&&Qt(0);break}if(G||Qt(2),de<256)t[d++]=de;else if(de==256){he=h,f=null;break}else{var q=de-254;if(de>264){var R=de-257,ie=fl[R];q=Dn(s,h,(1<<ie)-1)+Ad[R],h+=ie}var ce=m[_h(s,h)&$],Ce=ce>>4;ce||Qt(3),h+=ce&15;var W=by[Ce];if(Ce>3){var ie=dl[Ce];W+=_h(s,h)&(1<<ie)-1,h+=ie}if(h>g){l&&Qt(0);break}a&&c(d+131072);var ue=d+q;if(d<W){var H=r-W,se=Math.min(W,ue);for(H+d<0&&Qt(3);d<se;++d)t[d]=n[H+d]}for(;d<ue;++d)t[d]=t[d-W]}}e.l=f,e.p=he,e.b=d,e.f=u,f&&(u=1,e.m=_,e.d=m,e.n=p)}while(!u);return d!=t.length&&o?so(t,0,d):t.subarray(0,d)},mi=function(s,e,t){t<<=e&7;var n=e/8|0;s[n]|=t,s[n+1]|=t>>8},to=function(s,e,t){t<<=e&7;var n=e/8|0;s[n]|=t,s[n+1]|=t>>8,s[n+2]|=t>>16},xh=function(s,e){for(var t=[],n=0;n<s.length;++n)s[n]&&t.push({s:n,f:s[n]});var i=t.length,r=t.slice();if(!i)return{t:Rd,l:0};if(i==1){var o=new Rt(t[0].s+1);return o[t[0].s]=1,{t:o,l:1}}t.sort(function(b,w){return b.f-w.f}),t.push({s:-1,f:25001});var a=t[0],l=t[1],c=0,u=1,h=2;for(t[0]={s:-1,f:a.f+l.f,l:a,r:l};u!=i-1;)a=t[t[c].f<t[h].f?c++:h++],l=t[c!=u&&t[c].f<t[h].f?c++:h++],t[u++]={s:-1,f:a.f+l.f,l:a,r:l};for(var d=r[0].s,n=1;n<i;++n)r[n].s>d&&(d=r[n].s);var f=new pn(d+1),m=bh(t[u-1],f,0);if(m>e){var n=0,_=0,p=m-e,g=1<<p;for(r.sort(function(w,y){return f[y.s]-f[w.s]||w.f-y.f});n<i;++n){var S=r[n].s;if(f[S]>e)_+=g-(1<<m-f[S]),f[S]=e;else break}for(_>>=p;_>0;){var v=r[n].s;f[v]<e?_-=1<<e-f[v]++-1:++n}for(;n>=0&&_;--n){var x=r[n].s;f[x]==e&&(--f[x],++_)}m=e}return{t:new Rt(f),l:m}},bh=function(s,e,t){return s.s==-1?Math.max(bh(s.l,e,t+1),bh(s.r,e,t+1)):e[s.s]=t},xd=function(s){for(var e=s.length;e&&!s[--e];);for(var t=new pn(++e),n=0,i=s[0],r=1,o=function(l){t[n++]=l},a=1;a<=e;++a)if(s[a]==i&&a!=e)++r;else{if(!i&&r>2){for(;r>138;r-=138)o(32754);r>2&&(o(r>10?r-11<<5|28690:r-3<<5|12305),r=0)}else if(r>3){for(o(i),--r;r>6;r-=6)o(8304);r>2&&(o(r-3<<5|8208),r=0)}for(;r--;)o(i);r=1,i=s[a]}return{c:t.subarray(0,n),n:e}},no=function(s,e){for(var t=0,n=0;n<e.length;++n)t+=s[n]*e[n];return t},Ed=function(s,e,t){var n=t.length,i=Eh(e+2);s[i]=n&255,s[i+1]=n>>8,s[i+2]=s[i]^255,s[i+3]=s[i+1]^255;for(var r=0;r<n;++r)s[i+r+4]=t[r];return(i+4+n)*8},yd=function(s,e,t,n,i,r,o,a,l,c,u){mi(e,u++,t),++i[256];for(var h=xh(i,15),d=h.t,f=h.l,m=xh(r,15),_=m.t,p=m.l,g=xd(d),S=g.c,v=g.n,x=xd(_),b=x.c,w=x.n,y=new pn(19),M=0;M<S.length;++M)++y[S[M]&31];for(var M=0;M<b.length;++M)++y[b[M]&31];for(var C=xh(y,7),E=C.t,R=C.l,D=19;D>4&&!E[vh[D-1]];--D);var k=c+5<<3,P=no(i,Ii)+no(r,io)+o,O=no(i,d)+no(r,_)+o+14+3*D+no(y,E)+2*y[16]+3*y[17]+7*y[18];if(l>=0&&k<=P&&k<=O)return Ed(e,u,s.subarray(l,l+c));var G,U,ee,W;if(mi(e,u,1+(O<P)),u+=2,O<P){G=Yn(d,f,0),U=d,ee=Yn(_,p,0),W=_;var K=Yn(E,R,0);mi(e,u,v-257),mi(e,u+5,w-1),mi(e,u+10,D-4),u+=14;for(var M=0;M<D;++M)mi(e,u+3*M,E[vh[M]]);u+=3*D;for(var $=[S,b],he=0;he<2;++he)for(var de=$[he],M=0;M<de.length;++M){var q=de[M]&31;mi(e,u,K[q]),u+=E[q],q>15&&(mi(e,u,de[M]>>5&127),u+=de[M]>>12)}}else G=wy,U=Ii,ee=Ty,W=io;for(var M=0;M<a;++M){var ie=n[M];if(ie>255){var q=ie>>18&31;to(e,u,G[q+257]),u+=U[q+257],q>7&&(mi(e,u,ie>>23&31),u+=fl[q]);var ce=ie&31;to(e,u,ee[ce]),u+=W[ce],ce>3&&(to(e,u,ie>>5&8191),u+=dl[ce])}else to(e,u,G[ie]),u+=U[ie]}return to(e,u,G[256]),u+U[256]},Py=new Th([65540,131080,131088,131104,262176,1048704,1048832,2114560,2117632]),Rd=new Rt(0),Iy=function(s,e,t,n,i,r){var o=r.z||s.length,a=new Rt(n+o+5*(1+Math.ceil(o/7e3))+i),l=a.subarray(n,a.length-i),c=r.l,u=(r.r||0)&7;if(e){u&&(l[0]=r.r>>3);for(var h=Py[e-1],d=h>>13,f=h&8191,m=(1<<t)-1,_=r.p||new pn(32768),p=r.h||new pn(m+1),g=Math.ceil(t/3),S=2*g,v=function(N){return(s[N]^s[N+1]<<g^s[N+2]<<S)&m},x=new Th(25e3),b=new pn(288),w=new pn(32),y=0,M=0,C=r.i||0,E=0,R=r.w||0,D=0;C+2<o;++C){var k=v(C),P=C&32767,O=p[k];if(_[P]=O,p[k]=P,R<=C){var G=o-C;if((y>7e3||E>24576)&&(G>423||!c)){u=yd(s,l,0,x,b,w,M,E,D,C-D,u),E=y=M=0,D=C;for(var U=0;U<286;++U)b[U]=0;for(var U=0;U<30;++U)w[U]=0}var ee=2,W=0,K=f,$=P-O&32767;if(G>2&&k==v(C-$))for(var he=Math.min(d,G)-1,de=Math.min(32767,C),q=Math.min(258,G);$<=de&&--K&&P!=O;){if(s[C+ee]==s[C+ee-$]){for(var ie=0;ie<q&&s[C+ie]==s[C+ie-$];++ie);if(ie>ee){if(ee=ie,W=$,ie>he)break;for(var ce=Math.min($,ie-2),Ce=0,U=0;U<ce;++U){var ue=C-$+U&32767,H=_[ue],se=ue-H&32767;se>Ce&&(Ce=se,O=ue)}}}P=O,O=_[P],$+=P-O&32767}if(W){x[E++]=268435456|Sh[ee]<<18|_d[W];var fe=Sh[ee]&31,Le=_d[W]&31;M+=fl[fe]+dl[Le],++b[257+fe],++w[Le],R=C+ee,++y}else x[E++]=s[C],++b[s[C]]}}for(C=Math.max(C,R);C<o;++C)x[E++]=s[C],++b[s[C]];u=yd(s,l,c,x,b,w,M,E,D,C-D,u),c||(r.r=u&7|l[u/8|0]<<3,u-=7,r.h=p,r.p=_,r.i=C,r.w=R)}else{for(var C=r.w||0;C<o+c;C+=65535){var V=C+65535;V>=o&&(l[u/8|0]=c,V=o),u=Ed(l,u+1,s.subarray(C,V))}r.i=o}return so(a,0,n+Eh(u)+i)},Dy=(function(){for(var s=new Int32Array(256),e=0;e<256;++e){for(var t=e,n=9;--n;)t=(t&1&&-306674912)^t>>>1;s[e]=t}return s})(),Ly=function(){var s=-1;return{p:function(e){for(var t=s,n=0;n<e.length;++n)t=Dy[t&255^e[n]]^t>>>8;s=t},d:function(){return~s}}};var Ny=function(s,e,t,n,i){if(!i&&(i={l:1},e.dictionary)){var r=e.dictionary.subarray(-32768),o=new Rt(r.length+s.length);o.set(r),o.set(s,r.length),s=o,i.w=r.length}return Iy(s,e.level==null?6:e.level,e.mem==null?i.l?Math.ceil(Math.max(8,Math.min(13,Math.log(s.length)))*1.5):20:12+e.mem,t,n,i)},Cd=function(s,e){var t={};for(var n in s)t[n]=s[n];for(var n in e)t[n]=e[n];return t};var qn=function(s,e){return s[e]|s[e+1]<<8},bn=function(s,e){return(s[e]|s[e+1]<<8|s[e+2]<<16|s[e+3]<<24)>>>0},yh=function(s,e){return bn(s,e)+bn(s,e+4)*4294967296},Yt=function(s,e,t){for(;t;++e)s[e]=t,t>>>=8};function Uy(s,e){return Ny(s,e||{},0,0)}function Fy(s,e){return Cy(s,{i:2},e&&e.out,e&&e.dictionary)}var Pd=function(s,e,t,n){for(var i in s){var r=s[i],o=e+i,a=n;Array.isArray(r)&&(a=Cd(n,r[1]),r=r[0]),ArrayBuffer.isView(r)?t[o]=[r,a]:(t[o+="/"]=[new Rt(0),a],Pd(r,o,t,n))}},vd=typeof TextEncoder<"u"&&new TextEncoder,wh=typeof TextDecoder<"u"&&new TextDecoder,Oy=0;try{wh.decode(Rd,{stream:!0}),Oy=1}catch{}var By=function(s){for(var e="",t=0;;){var n=s[t++],i=(n>127)+(n>223)+(n>239);if(t+i>s.length)return{s:e,r:so(s,t-1)};i?i==3?(n=((n&15)<<18|(s[t++]&63)<<12|(s[t++]&63)<<6|s[t++]&63)-65536,e+=String.fromCharCode(55296|n>>10,56320|n&1023)):i&1?e+=String.fromCharCode((n&31)<<6|s[t++]&63):e+=String.fromCharCode((n&15)<<12|(s[t++]&63)<<6|s[t++]&63):e+=String.fromCharCode(n)}};function Sd(s,e){if(e){for(var t=new Rt(s.length),n=0;n<s.length;++n)t[n]=s.charCodeAt(n);return t}if(vd)return vd.encode(s);for(var i=s.length,r=new Rt(s.length+(s.length>>1)),o=0,a=function(u){r[o++]=u},n=0;n<i;++n){if(o+5>r.length){var l=new Rt(o+8+(i-n<<1));l.set(r),r=l}var c=s.charCodeAt(n);c<128||e?a(c):c<2048?(a(192|c>>6),a(128|c&63)):c>55295&&c<57344?(c=65536+(c&1047552)|s.charCodeAt(++n)&1023,a(240|c>>18),a(128|c>>12&63),a(128|c>>6&63),a(128|c&63)):(a(224|c>>12),a(128|c>>6&63),a(128|c&63))}return so(r,0,o)}function zy(s,e){if(e){for(var t="",n=0;n<s.length;n+=16384)t+=String.fromCharCode.apply(null,s.subarray(n,n+16384));return t}else{if(wh)return wh.decode(s);var i=By(s),r=i.s,t=i.r;return t.length&&Qt(8),r}}var ky=function(s,e){return e+30+qn(s,e+26)+qn(s,e+28)},Vy=function(s,e,t){var n=qn(s,e+28),i=qn(s,e+30),r=zy(s.subarray(e+46,e+46+n),!(qn(s,e+8)&2048)),o=e+46+n,a=Gy(s,o,i,t,bn(s,e+20),bn(s,e+24),bn(s,e+42)),l=a[0],c=a[1],u=a[2];return[qn(s,e+10),l,c,r,o+i+qn(s,e+32),u]},Gy=function(s,e,t,n,i,r,o){var a=i==4294967295,l=r==4294967295,c=o==4294967295,u=e+t,h=a+l+c;if(n&&h){for(;e+4<u;e+=4+qn(s,e+2))if(qn(s,e)==1)return[a?yh(s,e+4+8*l):i,l?yh(s,e+4):r,c?yh(s,e+4+8*(l+a)):o,1];n<2&&Qt(13)}return[i,r,o,0]},Ah=function(s){var e=0;if(s)for(var t in s){var n=s[t].length;n>65535&&Qt(9),e+=n+4}return e},Md=function(s,e,t,n,i,r,o,a){var l=n.length,c=t.extra,u=a&&a.length,h=Ah(c);Yt(s,e,o!=null?33639248:67324752),e+=4,o!=null&&(s[e++]=20,s[e++]=t.os),s[e]=20,e+=2,s[e++]=t.flag<<1|(r<0&&8),s[e++]=i&&8,s[e++]=t.compression&255,s[e++]=t.compression>>8;var d=new Date(t.mtime==null?Date.now():t.mtime),f=d.getFullYear()-1980;if((f<0||f>119)&&Qt(10),Yt(s,e,f<<25|d.getMonth()+1<<21|d.getDate()<<16|d.getHours()<<11|d.getMinutes()<<5|d.getSeconds()>>1),e+=4,r!=-1&&(Yt(s,e,t.crc),Yt(s,e+4,r<0?-r-2:r),Yt(s,e+8,t.size)),Yt(s,e+12,l),Yt(s,e+14,h),e+=16,o!=null&&(Yt(s,e,u),Yt(s,e+6,t.attrs),Yt(s,e+10,o),e+=14),s.set(n,e),e+=l,h)for(var m in c){var _=c[m],p=_.length;Yt(s,e,+m),Yt(s,e+2,p),s.set(_,e+4),e+=4+p}return u&&(s.set(a,e),e+=u),e},Hy=function(s,e,t,n,i){Yt(s,e,101010256),Yt(s,e+8,t),Yt(s,e+10,t),Yt(s,e+12,n),Yt(s,e+16,i)};function Id(s,e){e||(e={});var t={},n=[];Pd(s,"",t,e);var i=0,r=0;for(var o in t){var a=t[o],l=a[0],c=a[1],u=c.level==0?0:8,h=Sd(o),d=h.length,f=c.comment,m=f&&Sd(f),_=m&&m.length,p=Ah(c.extra);d>65535&&Qt(11);var g=u?Uy(l,c):l,S=g.length,v=Ly();v.p(l),n.push(Cd(c,{size:l.length,crc:v.d(),c:g,f:h,m,u:d!=o.length||m&&f.length!=_,o:i,compression:u})),i+=30+d+p+S,r+=76+2*(d+p)+(_||0)+S}for(var x=new Rt(r+22),b=i,w=r-i,y=0;y<n.length;++y){var h=n[y];Md(x,h.o,h,h.f,h.u,h.c.length);var M=30+h.f.length+Ah(h.extra);x.set(h.c,h.o+M),Md(x,i,h,h.f,h.u,h.c.length,h.o,h.m),i+=16+M+(h.m?h.m.length:0)}return Hy(x,i,n.length,w,b),x}function pl(s,e){for(var t={},n=s.length-22;bn(s,n)!=101010256;--n)(!n||s.length-n>65558)&&Qt(13);var i=qn(s,n+8);if(!i)return{};var r=bn(s,n+16),o=bn(s,n-20)==117853008;if(o){var a=bn(s,n-12);o=bn(s,a)==101075792,o&&(i=bn(s,a+32),r=bn(s,a+48))}for(var l=e&&e.filter,c=0;c<i;++c){var u=Vy(s,r,o),h=u[0],d=u[1],f=u[2],m=u[3],_=u[4],p=u[5],g=ky(s,p);r=_,(!l||l({name:m,size:d,originalSize:f,compression:h}))&&(h?h==8?t[m]=Fy(s.subarray(g,g+d),{out:new Rt(f)}):Qt(14,"unknown compression type "+h):t[m]=so(s,g,g+d))}return t}var Wy=/^def\s+(?:(\w+)\s+)?"?([^"]+)"?$/,Xy=/^string\s+(\w+)$/,qy=/^(?:uniform\s+)?(\w+(?:\[\])?)\s+(.+)$/,ml=Symbol("valueMetadata"),gl={Attribute:1,Prim:6,Relationship:8},_l=class{parseText(e){e=this._preprocess(e);let t={},n=e.split(`
`),i=null,r=t,o=[t];for(let a of n)if(a.includes("=")){let l=this._findAssignmentOperator(a);if(l===-1){i=a.trim();continue}let c=a.slice(0,l).trim(),u=a.slice(l+1).trim();if(u.endsWith("{")){let h={};o.push(h),r[c]=h,r=h}else if(u.endsWith("(")){let h=u.slice(0,-1).trim();r[c]=h;let d={};r[ml]===void 0&&(r[ml]={}),r[ml][c]=d,o.push(d),r=d}else r[c]=u}else if(a.includes(":")&&!a.includes("=")){let l=a.indexOf(":"),c=a.slice(0,l).trim(),u=a.slice(l+1).trim();/^-?[\d.]+$/.test(c)&&(r[c]=u)}else if(a.endsWith("{")){i=a.slice(0,-1).trim()||i;let l=r[i]||{};o.push(l),r[i]=l,r=l}else if(a.endsWith("}")){if(o.pop(),o.length===0)continue;r=o[o.length-1]}else if(a.endsWith("(")){let l={};o.push(l),i=a.split("(")[0].trim()||i,r[i]=l,r=l}else a.endsWith(")")?(o.pop(),r=o[o.length-1]):a.trim()&&(i=a.trim());return t}_preprocess(e){e=this._stripBlockComments(e),e=this._collapseTripleQuotedStrings(e);let t=e.split(`
`),n=[],i=!1,r=0,o=0,a="";for(let l=0;l<t.length;l++){let c=t[l];c=this._stripInlineComment(c);let u=c.trim();if(i){a+=" "+u;for(let h of u)h==="["?r++:h==="]"?r--:h==="("&&r>0?o++:h===")"&&r>0&&o--;r===0&&o===0&&(n.push(a),a="",i=!1)}else{if(u.includes("=")){let h=this._findAssignmentOperator(u);if(h!==-1){let d=u.slice(h+1).trim(),f=0,m=0;for(let _ of d)_==="["?f++:_==="]"&&m++;if(f>m){i=!0,r=f-m,o=0,a=u;continue}}}n.push(u)}}return n.join(`
`)}_stripBlockComments(e){let t=/\/\*|"""|'''|["']/g,n="",i=0;for(;i<e.length;){t.lastIndex=i;let r=t.exec(e);if(r===null){n+=e.slice(i);break}let o=r.index,a=r[0];if(a==="/*"){n+=e.slice(i,o);let l=e.indexOf("*/",o+2);i=l===-1?e.length:l+2}else{let l=this._findStringEnd(e,o,a);n+=e.slice(i,l),i=l}}return n}_findStringEnd(e,t,n){if(n.length===3){let i=e.indexOf(n,t+3);return i===-1?e.length:i+3}for(let i=t+1;i<e.length;i++){let r=e[i];if(r==="\\")i++;else{if(r===n)return i+1;if(r===`
`)return i}}return e.length}_collapseTripleQuotedStrings(e){let t=/'''|"""/g,n="",i=0;for(;i<e.length;){t.lastIndex=i;let r=t.exec(e);if(r===null){n+=e.slice(i);break}let o=r.index,a=r[0];n+=e.slice(i,o);let l=e.indexOf(a,o+3),c=e.slice(o+3,l===-1?e.length:l);if(n+=a+c.replace(/\r/g,"").replace(/\n/g,"\\n"),l===-1)break;n+=a,i=l+3}return n}_stripInlineComment(e){if(e.trim().startsWith("#usda"))return e;let t=!1,n=null,i=!1;for(let r=0;r<e.length;r++){let o=e[r];if(i){i=!1;continue}if(o==="\\"){i=!0;continue}if(!t&&(o==='"'||o==="'"))t=!0,n=o;else if(t&&o===n)t=!1,n=null;else if(!t&&o==="#")return e.slice(0,r).trimEnd()}return e}_findAssignmentOperator(e){let t=!1,n=null,i=!1;for(let r=0;r<e.length;r++){let o=e[r];if(i){i=!1;continue}if(o==="\\"){i=!0;continue}if(!t&&(o==='"'||o==="'"))t=!0,n=o;else if(t&&o===n)t=!1,n=null;else if(!t&&o==="=")return r}return-1}parseData(e){let t=this.parseText(e),n={},i={};if("#usda 1.0"in t){let o=t["#usda 1.0"];o.upAxis&&(i.upAxis=o.upAxis.replace(/"/g,"")),o.defaultPrim&&(i.defaultPrim=o.defaultPrim.replace(/"/g,"")),o.metersPerUnit!==void 0&&(i.metersPerUnit=parseFloat(o.metersPerUnit)),o.framesPerSecond!==void 0&&(i.framesPerSecond=parseFloat(o.framesPerSecond)),o.timeCodesPerSecond!==void 0&&(i.timeCodesPerSecond=parseFloat(o.timeCodesPerSecond))}n["/"]={specType:gl.Prim,fields:i};let r=(o,a)=>{let l=[];for(let c in o){if(c==="#usda 1.0"||c==="variants")continue;let u=c.match(Wy);if(u){let h=u[1]||"",d=u[2],f=a==="/"?"/"+d:a+"/"+d;l.push(d);let m={typeName:h},_=o[c];this._extractPrimData(_,f,m,n,gl),n[f]={specType:gl.Prim,fields:m},r(_,f)}}l.length>0&&n[a]&&(n[a].fields.primChildren=l)};return r(t,"/"),this._inferSkelElementSize(n),{specsByPath:n}}_inferSkelElementSize(e){for(let t in e){let n=e[t];if(n.specType!==gl.Prim||n.fields.typeName!=="Mesh")continue;let i=e[t+".points"];if(!i||!i.fields.default)continue;let r=i.fields.default.length/3;r!==0&&(this._inferElementSize(e[t+".primvars:skel:jointIndices"],r),this._inferElementSize(e[t+".primvars:skel:jointWeights"],r))}}_inferElementSize(e,t){if(!e||e.fields.elementSize!==void 0||!e.fields.default)return;let n=e.fields.default.length;n>0&&n%t===0&&(e.fields.elementSize=n/t)}_extractPrimData(e,t,n,i,r){if(!(!e||typeof e!="object"))for(let o in e){if(o.startsWith("def "))continue;if(o==="prepend references"){n.references=[e[o]];continue}if(o==="payload"){n.payload=e[o];continue}if(o==="variants"){let l={},c=e[o];for(let u in c){let h=u.match(Xy);if(h){let d=h[1],f=c[u].replace(/"/g,"");l[d]=f}}Object.keys(l).length>0&&(n.variantSelection=l);continue}if(o.startsWith("rel ")){let l=o.slice(4),c=t+"."+l,u=e[o].replace(/[<>]/g,""),h=e[ml]?.[o],d={targetPaths:[u]};h?.bindMaterialAs!==void 0&&(d.bindMaterialAs=this._parseString(String(h.bindMaterialAs).trim())),i[c]={specType:r.Relationship,fields:d};continue}if(o.includes("xformOpOrder")){let l=e[o].replace(/[\[\]]/g,"").split(",").map(c=>c.trim().replace(/"/g,""));n.xformOpOrder=l;continue}let a=o.match(qy);if(a){let l=a[1],c=a[2],u=e[o];if(c.endsWith(".connect")){let h=c.slice(0,-8),d=t+"."+h,f=String(u).trim();f.startsWith("<")&&(f=f.slice(1)),f.endsWith(">")&&(f=f.slice(0,-1)),i[d]||(i[d]={specType:r.Attribute,fields:{typeName:l}}),i[d].fields.connectionPaths=[f];continue}if(c.endsWith(".timeSamples")&&typeof u=="object"){let h=c.slice(0,-12),d=t+"."+h,f=[],m=[];for(let p in u){let g=parseFloat(p);isNaN(g)||(f.push(g),m.push(this._parseAttributeValue(l,u[p])))}let _=f.map((p,g)=>({t:p,v:m[g]})).sort((p,g)=>p.t-g.t);i[d]={specType:r.Attribute,fields:{timeSamples:{times:_.map(p=>p.t),values:_.map(p=>p.v)},typeName:l}}}else{let h=this._parseAttributeValue(l,u),d=t+"."+c;i[d]?(i[d].fields.default=h,i[d].fields.typeName=l):i[d]={specType:r.Attribute,fields:{default:h,typeName:l}}}}}}_parseAttributeValue(e,t){if(t==null)return;let n=String(t).trim();if(e.endsWith("[]")){let i;try{let r=n.replace(/\(/g,"[").replace(/\)/g,"]");r.endsWith(",")&&(r=r.slice(0,-1));let o=JSON.parse(r);i=Array.isArray(o)&&Array.isArray(o[0])?o.flat():o}catch{i=n.replace(/[\[\]]/g,"").split(",").map(a=>{let l=a.trim(),c=parseFloat(l);return isNaN(c)?l.replace(/"/g,""):c})}if(e.startsWith("quat"))for(let r=0;r<i.length;r+=4){let o=i[r];i[r]=i[r+1],i[r+1]=i[r+2],i[r+2]=i[r+3],i[r+3]=o}return i}if(e.includes("3")||e.includes("2")||e.includes("4"))return n.replace(/[()]/g,"").split(",").map(o=>parseFloat(o.trim()));if(e.startsWith("quat")){let r=n.replace(/[()]/g,"").split(",").map(o=>parseFloat(o.trim()));return[r[1],r[2],r[3],r[0]]}return e.includes("matrix")?n.replace(/[()]/g,"").split(",").map(o=>parseFloat(o.trim())):e==="float"||e==="double"||e==="int"?parseFloat(n):e==="string"||e==="token"?this._parseString(n):e==="asset"?n.replace(/@/g,"").replace(/"/g,""):this._parseString(n)}_parseString(e){(e.startsWith('"')&&e.endsWith('"')||e.startsWith("'")&&e.endsWith("'"))&&(e=e.slice(1,-1));let t="",n=0;for(;n<e.length;)if(e[n]==="\\"&&n+1<e.length){let i=e[n+1];switch(i){case"n":t+=`
`;break;case"t":t+="	";break;case"r":t+="\r";break;case"\\":t+="\\";break;case'"':t+='"';break;case"'":t+="'";break;default:t+=i;break}n+=2}else t+=e[n],n++;return t}};var Rh=new TextDecoder,Ld=new Float32Array(32);for(let s=0;s<32;s++)Ld[s]=Math.pow(2,s-15);var Yy=Math.pow(2,-14),De={Invalid:0,Bool:1,UChar:2,Int:3,UInt:4,Int64:5,UInt64:6,Half:7,Float:8,Double:9,String:10,Token:11,AssetPath:12,Matrix2d:13,Matrix3d:14,Matrix4d:15,Quatd:16,Quatf:17,Quath:18,Vec2d:19,Vec2f:20,Vec2h:21,Vec2i:22,Vec3d:23,Vec3f:24,Vec3h:25,Vec3i:26,Vec4d:27,Vec4f:28,Vec4h:29,Vec4i:30,Dictionary:31,TokenListOp:32,StringListOp:33,PathListOp:34,ReferenceListOp:35,IntListOp:36,Int64ListOp:37,UIntListOp:38,UInt64ListOp:39,PathVector:40,TokenVector:41,Specifier:42,Permission:43,Variability:44,VariantSelectionMap:45,TimeSamples:46,Payload:47,DoubleVector:48,LayerOffsetVector:49,StringVector:50,ValueBlock:51,Value:52,UnregisteredValue:53,UnregisteredValueListOp:54,PayloadListOp:55,TimeCode:56,PathExpression:57,Relocates:58,Spline:59,AnimationBlock:60},Zy=4294967295,Jy=105,$y=116;function Dd(s,e,t,n,i,r){for(;e<t;){let o=s[e++];if(e>t)break;let a=o>>4;if(a===15){let h;do{if(e>=t)break;h=s[e++],a+=h}while(h===255&&e<t)}if(a>0){e+a>t&&(a=t-e);for(let h=0;h<a&&!(i>=r);h++)n[i++]=s[e++]}if(e>=t||e+2>t)break;let l=s[e++]|s[e++]<<8;if(l===0)break;let c=(o&15)+4;if(c===19){let h;do{if(e>=t)break;h=s[e++],c+=h}while(h===255&&e<t)}let u=i-l;if(u<0)break;for(let h=0;h<c&&!(i>=r);h++)n[i++]=n[u+h]}return i}function Ch(s,e){let t=new Uint8Array(e),n=s[0];if(n===0)return Dd(s,1,s.length,t,0,e),t;{let r=1,o=[];for(let c=0;c<n;c++){let u=(s[r]|s[r+1]<<8|s[r+2]<<16|s[r+3]<<24)>>>0;o.push(u),r+=4}let a=r,l=0;for(let c=0;c<n;c++){let u=o[c],h=Math.min(65536,e-l);Dd(s,a,a+u,t,l,l+h),a+=u,l+=h}return t}}function Ln(s,e){let t=e*4+(e*2+7>>3)+4,n=Ch(new Uint8Array(s),t);return Ky(n,e)}function Ky(s,e){let t=new DataView(s.buffer,s.byteOffset,s.byteLength),n=0,i=t.getInt32(n,!0);n+=4;let r=e*2+7>>3,o=n,a=n+r,l=new Int32Array(e),c=0,u=o,h=a;for(let d=0;d<e;){let f=s[u++];for(let m=0;m<4&&d<e;m++,d++){let _=f>>m*2&3,p=0;switch(_){case 0:p=i;break;case 1:p=t.getInt8(h),h+=1;break;case 2:p=t.getInt16(h,!0),h+=2;break;case 3:p=t.getInt32(h,!0),h+=4;break}c+=p,l[d]=c}}return l}var Ph=class{constructor(e){this.buffer=e,this.view=new DataView(e),this.offset=0}seek(e){this.offset=e}tell(){return this.offset}readUint8(){let e=this.view.getUint8(this.offset);return this.offset+=1,e}readInt8(){let e=this.view.getInt8(this.offset);return this.offset+=1,e}readUint16(){let e=this.view.getUint16(this.offset,!0);return this.offset+=2,e}readInt16(){let e=this.view.getInt16(this.offset,!0);return this.offset+=2,e}readUint32(){let e=this.view.getUint32(this.offset,!0);return this.offset+=4,e}readInt32(){let e=this.view.getInt32(this.offset,!0);return this.offset+=4,e}readUint64(){let e=this.view.getUint32(this.offset,!0),t=this.view.getUint32(this.offset+4,!0);return this.offset+=8,t*4294967296+e}readInt64(){let e=this.view.getUint32(this.offset,!0),t=this.view.getInt32(this.offset+4,!0);return this.offset+=8,t*4294967296+e}readFloat32(){let e=this.view.getFloat32(this.offset,!0);return this.offset+=4,e}readFloat64(){let e=this.view.getFloat64(this.offset,!0);return this.offset+=8,e}readBytes(e){let t=new Uint8Array(this.buffer,this.offset,e);return this.offset+=e,t}readString(e){let t=this.readBytes(e),n=0;for(;n<e&&t[n]!==0;)n++;return Rh.decode(t.subarray(0,n))}},ts=class{constructor(e,t){this.lo=e,this.hi=t}get isArray(){return(this.hi&2147483648)!==0}get isInlined(){return(this.hi&1073741824)!==0}get isCompressed(){return(this.hi&536870912)!==0}get typeEnum(){return this.hi>>16&255}get payload(){return this.lo+(this.hi&65535)*4294967296}getInlinedValue(){return this.lo}},xl=class{parseData(e){this.buffer=e instanceof ArrayBuffer?e:e.buffer,this.reader=new Ph(this.buffer),this.version={major:0,minor:0,patch:0},this._conversionBuffer=new ArrayBuffer(4),this._conversionView=new DataView(this._conversionBuffer),this._readBootstrap(),this._readTOC(),this._readTokens(),this._readStrings(),this._readFields(),this._readFieldSets(),this._readPaths(),this._readSpecs(),this.specsByPath={};for(let t of this.specs){let n=this.paths[t.pathIndex];if(!n)continue;let i=this._getFieldsForSpec(t);this.specsByPath[n]={specType:t.specType,fields:i}}return{specsByPath:this.specsByPath}}_readBootstrap(){let e=this.reader;if(e.seek(0),e.readString(8)!=="PXR-USDC")throw new Error("THREE.USDCParser: Not a valid USDC file.");this.version.major=e.readUint8(),this.version.minor=e.readUint8(),this.version.patch=e.readUint8(),e.readBytes(5),this.tocOffset=e.readUint64()}_readTOC(){let e=this.reader;e.seek(this.tocOffset);let t=e.readUint64();this.sections={};for(let n=0;n<t;n++){let i=e.readString(16),r=e.readUint64(),o=e.readUint64();this.sections[i]={start:r,size:o}}}_readTokens(){let e=this.sections.TOKENS;if(!e)return;let t=this.reader;t.seek(e.start);let n=t.readUint64();if(this.tokens=[],this.version.major===0&&this.version.minor<4){let i=t.readUint64(),r=t.readBytes(i),o=0;for(let a=0;a<n;a++){let l=o;for(;l<r.length&&r[l]!==0;)l++;this.tokens.push(Rh.decode(r.subarray(o,l))),o=l+1}}else{let i=t.readUint64(),r=t.readUint64(),o=t.readBytes(r),a=Ch(o,i),l=0;for(let c=0;c<n;c++){let u=l;for(;u<a.length&&a[u]!==0;)u++;this.tokens.push(Rh.decode(a.subarray(l,u))),l=u+1}}}_readStrings(){let e=this.sections.STRINGS;if(!e){this.strings=[];return}let t=this.reader;t.seek(e.start);let n=t.readUint64();this.strings=[];for(let i=0;i<n;i++)this.strings.push(t.readUint32())}_readFields(){let e=this.sections.FIELDS;if(!e)return;let t=this.reader;if(t.seek(e.start),this.fields=[],this.version.major===0&&this.version.minor<4){let n=Math.floor(e.size/12);for(let i=0;i<n;i++){let r=t.readUint32(),o=t.readUint32(),a=t.readUint32();this.fields.push({tokenIndex:r,valueRep:new ts(o,a)})}}else{let n=t.readUint64(),i=t.readUint64(),r=t.readBytes(i),o=Ln(r.buffer.slice(r.byteOffset,r.byteOffset+i),n),a=t.readUint64(),l=t.readBytes(a),c=Ch(l,n*8),u=new DataView(c.buffer,c.byteOffset,c.byteLength);for(let h=0;h<n;h++){let d=u.getUint32(h*8,!0),f=u.getUint32(h*8+4,!0);this.fields.push({tokenIndex:o[h],valueRep:new ts(d,f)})}}}_readFieldSets(){let e=this.sections.FIELDSETS;if(!e)return;let t=this.reader;if(t.seek(e.start),this.fieldSets=[],this.version.major===0&&this.version.minor<4){let n=Math.floor(e.size/4);for(let i=0;i<n;i++)this.fieldSets.push(t.readUint32())}else{let n=t.readUint64(),i=t.readUint64(),r=t.readBytes(i),o=Ln(r.buffer.slice(r.byteOffset,r.byteOffset+i),n);for(let a=0;a<n;a++)this.fieldSets.push(o[a])}}_readPaths(){let e=this.sections.PATHS;if(!e)return;let t=this.reader;t.seek(e.start);let n=t.readUint64();if(this.paths=new Array(n).fill(""),this.version.major===0&&this.version.minor<4)this._readPathsRecursive("");else{t.readUint64();let i=t.readUint64(),r=t.readBytes(i),o=Ln(r.buffer.slice(r.byteOffset,r.byteOffset+i),n),a=t.readUint64(),l=t.readBytes(a),c=Ln(l.buffer.slice(l.byteOffset,l.byteOffset+a),n),u=t.readUint64(),h=t.readBytes(u),d=Ln(h.buffer.slice(h.byteOffset,h.byteOffset+u),n);this._buildPathsFromCompressed(o,c,d)}}_readPathsRecursive(e,t=0){let n=this.reader;if(t>1e3)return;let i=n.readUint32(),r=n.readUint32(),o=n.readUint8(),a=(o&1)!==0,l=(o&2)!==0,c=(o&4)!==0,u;if(e==="")u="/";else{let h=this.tokens[r]||"";c?u=e+"."+h:u=e==="/"?"/"+h:e+"/"+h}if(this.paths[i]=u,a&&l){let h=n.readUint64();this._readPathsRecursive(u,t+1),n.seek(h),this._readPathsRecursive(e,t+1)}else a?this._readPathsRecursive(u,t+1):l&&this._readPathsRecursive(e,t+1)}_buildPathsFromCompressed(e,t,n){let i=(r,o)=>{let a=r;for(;a<e.length;){let l=a++,c=e[l],u=t[l],h=n[l],d;if(o==="")d="/",o=d;else{let _=this.tokens[Math.abs(u)]||"";u<0?d=o+"."+_:d=o==="/"?"/"+_:o+"/"+_}this.paths[c]=d;let f=h>0||h===-1,m=h>=0;if(f){if(m){let _=l+h;i(_,o)}o=d}else if(!m)break}};i(0,"")}_readSpecs(){let e=this.sections.SPECS;if(!e)return;let t=this.reader;if(t.seek(e.start),this.specs=[],this.version.major===0&&this.version.minor<4){let n=this.version.minor===0&&this.version.patch===1?16:12,i=Math.floor(e.size/n);for(let r=0;r<i;r++){let o=t.readUint32(),a=t.readUint32(),l=t.readUint32();n===16&&t.readUint32(),this.specs.push({pathIndex:o,fieldSetIndex:a,specType:l})}}else{let n=t.readUint64(),i=t.readUint64(),r=t.readBytes(i),o=Ln(r.buffer.slice(r.byteOffset,r.byteOffset+i),n),a=t.readUint64(),l=t.readBytes(a),c=Ln(l.buffer.slice(l.byteOffset,l.byteOffset+a),n),u=t.readUint64(),h=t.readBytes(u),d=Ln(h.buffer.slice(h.byteOffset,h.byteOffset+u),n);for(let f=0;f<n;f++)this.specs.push({pathIndex:o[f],fieldSetIndex:c[f],specType:d[f]})}}_readValue(e){let t=e.typeEnum,n=e.isArray,i=e.isInlined;if(t===De.TimeSamples)return this._readTimeSamples(e);if(i)return this._readInlinedValue(e);let r=e.payload;if(r===0&&n)return[];if(r<0||r>=this.buffer.byteLength)throw new RangeError("USDCParser: Invalid payload offset "+r+" for type "+t+".");let o=this.reader.tell();this.reader.seek(r);let a;return n?a=this._readArrayValue(e):a=this._readScalarValue(t),this.reader.seek(o),a}_readInlinedValue(e){let t=e.typeEnum,n=e.getInlinedValue(),i=this._conversionView;switch(t){case De.Bool:return n!==0;case De.UChar:return n&255;case De.Int:case De.UInt:return n;case De.Float:return i.setUint32(0,n,!0),i.getFloat32(0,!0);case De.Double:return i.setUint32(0,n,!0),i.getFloat32(0,!0);case De.Token:return this.tokens[n]||"";case De.String:return this.tokens[this.strings[n]]||"";case De.AssetPath:return this.tokens[n]||"";case De.Specifier:return n;case De.Permission:case De.Variability:return n;case De.Vec2h:return i.setUint32(0,n,!0),[this._halfToFloat(i.getUint16(0,!0)),this._halfToFloat(i.getUint16(2,!0))];case De.Vec2f:case De.Vec2i:return i.setUint32(0,n,!0),[i.getInt8(0),i.getInt8(1)];case De.Vec3f:case De.Vec3i:return i.setUint32(0,n,!0),[i.getInt8(0),i.getInt8(1),i.getInt8(2)];case De.Vec4f:case De.Vec4i:return i.setUint32(0,n,!0),[i.getInt8(0),i.getInt8(1),i.getInt8(2),i.getInt8(3)];case De.Matrix2d:{i.setUint32(0,n,!0);let r=i.getInt8(0),o=i.getInt8(1);return[r,0,0,o]}case De.Matrix3d:{i.setUint32(0,n,!0);let r=i.getInt8(0),o=i.getInt8(1),a=i.getInt8(2);return[r,0,0,0,o,0,0,0,a]}case De.Matrix4d:{i.setUint32(0,n,!0);let r=i.getInt8(0),o=i.getInt8(1),a=i.getInt8(2),l=i.getInt8(3);return[r,0,0,0,0,o,0,0,0,0,a,0,0,0,0,l]}default:return n}}_readTimeSamples(e){let t=this.reader,n=e.payload,i=t.tell();t.seek(n);let r=t.tell(),o=t.readInt64();t.seek(r+o);let a=t.readUint32(),l=t.readUint32(),c=new ts(a,l),u=this._readValue(c),h=r+o+8;t.seek(h);let d=t.tell(),f=t.readInt64();t.seek(d+f);let m=t.readUint64(),_=[];for(let S=0;S<m;S++){let v=t.readUint32(),x=t.readUint32();_.push(new ts(v,x))}let p=[];for(let S=0;S<m;S++)p.push(this._readValue(_[S]));return t.seek(i),{times:u instanceof Float64Array?Array.from(u):Array.isArray(u)?u:[u],values:p}}_readScalarValue(e){let t=this.reader;switch(e){case De.Invalid:return null;case De.Bool:return t.readUint8()!==0;case De.UChar:return t.readUint8();case De.Int:return t.readInt32();case De.UInt:return t.readUint32();case De.Int64:return t.readInt64();case De.UInt64:return t.readUint64();case De.Half:return this._readHalf();case De.Float:return t.readFloat32();case De.Double:return t.readFloat64();case De.String:case De.Token:{let n=t.readUint32();return this.tokens[n]||""}case De.AssetPath:{let n=t.readUint32();return this.tokens[n]||""}case De.Vec2f:return[t.readFloat32(),t.readFloat32()];case De.Vec2d:return[t.readFloat64(),t.readFloat64()];case De.Vec2i:return[t.readInt32(),t.readInt32()];case De.Vec3f:return[t.readFloat32(),t.readFloat32(),t.readFloat32()];case De.Vec3d:return[t.readFloat64(),t.readFloat64(),t.readFloat64()];case De.Vec3i:return[t.readInt32(),t.readInt32(),t.readInt32()];case De.Vec4f:return[t.readFloat32(),t.readFloat32(),t.readFloat32(),t.readFloat32()];case De.Vec4d:return[t.readFloat64(),t.readFloat64(),t.readFloat64(),t.readFloat64()];case De.Quatf:return[t.readFloat32(),t.readFloat32(),t.readFloat32(),t.readFloat32()];case De.Quatd:return[t.readFloat64(),t.readFloat64(),t.readFloat64(),t.readFloat64()];case De.Matrix4d:{let n=[];for(let i=0;i<16;i++)n.push(t.readFloat64());return n}case De.TokenVector:{let n=t.readUint64(),i=[];for(let r=0;r<n;r++){let o=t.readUint32();i.push(this.tokens[o]||"")}return i}case De.PathVector:{let n=t.readUint64(),i=[];for(let r=0;r<n;r++){let o=t.readUint32();i.push(this.paths[o]||"")}return i}case De.DoubleVector:{let n=t.readUint64(),i=new Float64Array(n);for(let r=0;r<n;r++)i[r]=t.readFloat64();return i}case De.Dictionary:{let n=t.readUint64(),i={};for(let r=0;r<n;r++){let o=t.readUint32(),a=this.tokens[o],l=t.position,c=t.readInt64(),u=l+c,h=t.position;t.position=u;let d=t.readUint64(),f=new ts(d),m=null;f.isInlined?m=this._readInlinedValue(f):f.isArray?(t.position=f.payload,m=this._readArrayValue(f)):(t.position=f.payload,m=this._readScalarValue(f.typeEnum)),t.position=h,a!==void 0&&m!==null&&(i[a]=m)}return i}case De.TokenListOp:case De.StringListOp:case De.IntListOp:case De.Int64ListOp:case De.UIntListOp:case De.UInt64ListOp:return null;case De.PathListOp:{let n=t.readUint8(),i=(n&2)!==0,r=(n&4)!==0,o=(n&8)!==0,a=(n&16)!==0,l=(n&32)!==0,c=(n&64)!==0,u=()=>{let _=t.readUint64(),p=[];for(let g=0;g<_;g++){let S=t.readUint32();p.push(this.paths[S])}return p},h=null,d=null,f=null,m=null;return i&&(h=u()),r&&(d=u()),l&&(f=u()),c&&(m=u()),o&&u(),a&&u(),f&&f.length>0?f:h&&h.length>0?h:m&&m.length>0?m:d&&d.length>0?d:null}case De.VariantSelectionMap:{let n=t.readUint64(),i={};for(let r=0;r<n;r++){let o=t.readUint32(),a=t.readUint32(),l=this.tokens[this.strings[o]],c=this.tokens[this.strings[a]];l&&c&&(i[l]=c)}return i}default:return console.warn("USDCParser: Unsupported scalar type",e),null}}_readArrayValue(e){let t=this.reader,n=e.typeEnum,i=e.isCompressed,r;if(this.version.major===0&&this.version.minor<7?r=t.readUint32():r=t.readUint64(),!Number.isSafeInteger(r)||r<0)throw new RangeError("USDCParser: Invalid array size "+r+" for type "+n+".");if(r>2147483647)throw new RangeError("USDCParser: Array size "+r+" exceeds implementation limits.");if(r===0)return[];if(i)return this._readCompressedArray(n,r);switch(n){case De.Int:{let o=new Int32Array(r);for(let a=0;a<r;a++)o[a]=t.readInt32();return o}case De.UInt:{let o=new Uint32Array(r);for(let a=0;a<r;a++)o[a]=t.readUint32();return o}case De.Float:{let o=new Float32Array(r);for(let a=0;a<r;a++)o[a]=t.readFloat32();return o}case De.Double:{let o=new Float64Array(r);for(let a=0;a<r;a++)o[a]=t.readFloat64();return o}case De.Vec2f:{let o=new Float32Array(r*2);for(let a=0;a<r*2;a++)o[a]=t.readFloat32();return o}case De.Vec3f:{let o=new Float32Array(r*3);for(let a=0;a<r*3;a++)o[a]=t.readFloat32();return o}case De.Vec4f:{let o=new Float32Array(r*4);for(let a=0;a<r*4;a++)o[a]=t.readFloat32();return o}case De.Vec3h:{let o=new Float32Array(r*3);for(let a=0;a<r*3;a++)o[a]=this._readHalf();return o}case De.Quatf:{let o=new Float32Array(r*4);for(let a=0;a<r*4;a++)o[a]=t.readFloat32();return o}case De.Quath:{let o=new Float32Array(r*4);for(let a=0;a<r*4;a++)o[a]=this._readHalf();return o}case De.Matrix4d:{let o=new Float64Array(r*16);for(let a=0;a<r*16;a++)o[a]=t.readFloat64();return o}case De.Token:{let o=[];for(let a=0;a<r;a++){let l=t.readUint32();o.push(this.tokens[l]||"")}return o}case De.Half:{let o=new Float32Array(r);for(let a=0;a<r;a++)o[a]=this._readHalf();return o}default:return console.warn("USDCParser: Unsupported array type",n),[]}}_readCompressedArray(e,t){let n=this.reader;switch(e){case De.Int:case De.UInt:{let i=n.readUint64(),r=n.readBytes(i);return Ln(r.buffer.slice(r.byteOffset,r.byteOffset+i),t)}case De.Float:{let i=n.readInt8();if(i===Jy){let r=n.readUint64(),o=n.readBytes(r),a=Ln(o.buffer.slice(o.byteOffset,o.byteOffset+r),t),l=new Float32Array(t);for(let c=0;c<t;c++)l[c]=a[c];return l}else if(i===$y){let r=n.readUint32(),o=new Float32Array(r);for(let h=0;h<r;h++)o[h]=n.readFloat32();let a=n.readUint64(),l=n.readBytes(a),c=Ln(l.buffer.slice(l.byteOffset,l.byteOffset+a),t),u=new Float32Array(t);for(let h=0;h<t;h++)u[h]=o[c[h]];return u}return console.warn("USDCParser: Unknown float compression code",i),new Float32Array(t)}default:return console.warn("USDCParser: Unsupported compressed array type",e),[]}}_readHalf(){return this._halfToFloat(this.reader.readUint16())}_halfToFloat(e){let t=(e&32768)>>15,n=(e&31744)>>10,i=e&1023;return n===0?i===0?t?-0:0:(t?-1:1)*Yy*(i/1024):n===31?i?NaN:t?-1/0:1/0:(t?-1:1)*Ld[n]*(1+i/1024)}_getFieldsForSpec(e){let t={},n=e.fieldSetIndex,i=1e4,r=0;for(;n<this.fieldSets.length&&r<i;){let o=this.fieldSets[n];if(o===Zy||o===-1)break;let a=this.fields[o];if(a){let l=this.tokens[a.tokenIndex],c=this._readValue(a.valueRep);t[l]=c}n++,r++}return t}};var jy=/^(.+?)\/\{(\w+)=(\w+)\}\/(.+)$/,ns={Unknown:0,Attribute:1,Connection:2,Expression:3,Mapper:4,MapperArg:5,Prim:6,PseudoRoot:7,Relationship:8,RelationshipTarget:9,Variant:10,VariantSet:11},Nn={projection:"perspective",clippingRange:[1,1e6],horizontalAperture:20.955,verticalAperture:15.2908,horizontalApertureOffset:0,verticalApertureOffset:0,focalLength:50,focusDistance:0,fStop:0},Zn=class s{constructor(e){this.textureCache={},this.skinnedMeshes=[],this.manager=e,this.imageLoader=new zr(e),this.texturePromises=[]}compose(e,t={},n={},i=""){this.specsByPath=e.specsByPath,this.assets=t,this.externalVariantSelections=n,this.basePath=i,this.skinnedMeshes=[],this.skeletons={},this.texturePromises=[],this._buildIndexes();let r=this.specsByPath["/"],o=r?r.fields:{};this.fps=o.timeCodesPerSecond||o.framesPerSecond||24;let a=new ut;this._buildHierarchy(a,"/"),this._bindSkeletons();let l=Object.keys(this.skeletons);l.length===1&&(a.skeleton=this.skeletons[l[0]].skeleton),a.animations=this._buildAnimations();let c=o.metersPerUnit;return c!==void 0&&c!==1&&a.scale.setScalar(c),r&&r.fields&&r.fields.upAxis==="Z"&&(a.rotation.x=-Math.PI/2),a}applyTransform(e,t,n={}){let i={...t,...n},r=i.xformOpOrder;if(r&&r.length>0){let o=new ze,a=new ze,l=null;for(let c=0;c<r.length;c++){let u=r[c],h=u.startsWith("!invert!"),d=h?u.slice(8):u;if(d==="xformOp:transform"){let f=i["xformOp:transform"];f&&f.length===16&&(a.set(f[0],f[4],f[8],f[12],f[1],f[5],f[9],f[13],f[2],f[6],f[10],f[14],f[3],f[7],f[11],f[15]),h&&a.invert(),o.multiply(a))}else if(d==="xformOp:translate"){let f=i["xformOp:translate"];f&&(a.makeTranslation(f[0],f[1],f[2]),h&&a.invert(),o.multiply(a))}else if(d==="xformOp:translate:pivot"){let f=i["xformOp:translate:pivot"];f&&(a.makeTranslation(f[0],f[1],f[2]),h&&a.invert(),o.multiply(a))}else if(d==="xformOp:scale"){let f=i["xformOp:scale"];f&&(Array.isArray(f)?(a.makeScale(f[0],f[1],f[2]),l=[f[0],f[1],f[2]]):(a.makeScale(f,f,f),l=[f,f,f]),h&&a.invert(),o.multiply(a))}else if(d==="xformOp:rotateXYZ"){let f=i["xformOp:rotateXYZ"];if(f){let m=new Ut(f[0]*Math.PI/180,f[1]*Math.PI/180,f[2]*Math.PI/180,"ZYX");a.makeRotationFromEuler(m),h&&a.invert(),o.multiply(a)}}else if(d==="xformOp:rotateX"){let f=i["xformOp:rotateX"];f!==void 0&&(a.makeRotationX(f*Math.PI/180),h&&a.invert(),o.multiply(a))}else if(d==="xformOp:rotateY"){let f=i["xformOp:rotateY"];f!==void 0&&(a.makeRotationY(f*Math.PI/180),h&&a.invert(),o.multiply(a))}else if(d==="xformOp:rotateZ"){let f=i["xformOp:rotateZ"];f!==void 0&&(a.makeRotationZ(f*Math.PI/180),h&&a.invert(),o.multiply(a))}else if(d==="xformOp:orient"){let f=i["xformOp:orient"];if(f&&f.length===4){let m=new st(f[0],f[1],f[2],f[3]);a.makeRotationFromQuaternion(m),h&&a.invert(),o.multiply(a)}}}if(e.matrix.copy(o),e.matrix.decompose(e.position,e.quaternion,e.scale),l){let c=l[0]<0,u=l[1]<0,h=l[2]<0;(c?1:0)+(u?1:0)+(h?1:0)===3&&(e.scale.set(l[0],l[1],l[2]),e.quaternion.set(e.quaternion.x,-e.quaternion.y,e.quaternion.z,-e.quaternion.w))}return}if(i["xformOp:translate"]){let o=i["xformOp:translate"];e.position.set(o[0],o[1],o[2])}if(i["xformOp:translate:pivot"]){let o=i["xformOp:translate:pivot"];e.pivot=new L(o[0],o[1],o[2])}if(i["xformOp:scale"]){let o=i["xformOp:scale"];Array.isArray(o)?e.scale.set(o[0],o[1],o[2]):e.scale.set(o,o,o)}if(i["xformOp:rotateXYZ"]){let o=i["xformOp:rotateXYZ"];e.rotation.set(o[0]*Math.PI/180,o[1]*Math.PI/180,o[2]*Math.PI/180)}if(i["xformOp:orient"]){let o=i["xformOp:orient"];o.length===4&&e.quaternion.set(o[0],o[1],o[2],o[3])}}_buildIndexes(){this.childrenByPath=new Map,this.attributesByPrimPath=new Map,this.materialsByRoot=new Map,this.shadersByMaterialPath=new Map,this.geomSubsetsByMeshPath=new Map;for(let e in this.specsByPath){let t=this.specsByPath[e];if(t.specType===ns.Prim){let n=e.lastIndexOf("/");if(n>0){let r=e.slice(0,n),o=e.slice(n+1);this.childrenByPath.has(r)||this.childrenByPath.set(r,[]),this.childrenByPath.get(r).push({name:o,path:e})}else if(n===0&&e.length>1){let r=e.slice(1);this.childrenByPath.has("/")||this.childrenByPath.set("/",[]),this.childrenByPath.get("/").push({name:r,path:e})}let i=t.fields.typeName;if(i==="Material"){let r=e.split("/"),o=r.length>1?"/"+r[1]:"/";this.materialsByRoot.has(o)||this.materialsByRoot.set(o,[]),this.materialsByRoot.get(o).push(e)}if(i==="Shader"&&n>0){let r=e.slice(0,n);for(;r.length>0;){let o=this.specsByPath[r];if(o&&o.specType===ns.Prim&&o.fields.typeName==="Material"){this.shadersByMaterialPath.has(r)||this.shadersByMaterialPath.set(r,[]),this.shadersByMaterialPath.get(r).push(e);break}let a=r.lastIndexOf("/");if(a<=0)break;r=r.slice(0,a)}}if(i==="GeomSubset"&&n>0){let r=e.slice(0,n);this.geomSubsetsByMeshPath.has(r)||this.geomSubsetsByMeshPath.set(r,[]),this.geomSubsetsByMeshPath.get(r).push(e)}}else if(t.specType===ns.Attribute||t.specType===ns.Relationship){let n=e.lastIndexOf(".");if(n>0){let i=e.slice(0,n),r=e.slice(n+1);this.attributesByPrimPath.has(i)||this.attributesByPrimPath.set(i,new Map),this.attributesByPrimPath.get(i).set(r,t)}}}}_isDirectChild(e,t,n){if(!t.startsWith(n))return!1;let i=t.slice(n.length);return i.length===0||i.startsWith("{")?!1:!i.includes("/")}_buildHierarchy(e,t){let n=[],i=new Set,r=this.childrenByPath.get(t);if(r)for(let a of r)i.has(a.path)||(i.add(a.path),n.push(a));let o=this._getVariantPaths(t);for(let a of o){let l=this.childrenByPath.get(a);if(l)for(let c of l)i.has(c.path)||(i.add(c.path),n.push(c))}for(let{name:a,path:l}of n){let c=this.specsByPath[l];if(!c||c.specType!==ns.Prim)continue;let u=c.fields.typeName,h=this._getReferences(c);if(h.length>0){let d=this._getLocalVariantSelections(c.fields),f=[];for(let m of h){let _=this._resolveReference(m,d);_&&f.push(_)}if(f.length>0){let m=this._getAttributes(l);if(f.length===1){let p=this._findSingleMesh(f[0]);if(p&&(u==="Xform"||!u)){p.name=a,this.applyTransform(p,c.fields,m),this._applyMaterialBinding(p,l),e.add(p),this._buildHierarchy(p,l);continue}}let _=new At;_.name=a,this.applyTransform(_,c.fields,m);for(let p of f)for(;p.children.length>0;)_.add(p.children[0]);e.add(_),this._buildHierarchy(_,l);continue}}if(u==="SkelRoot"){let d=new At;d.name=a,d.userData.isSkelRoot=!0;let f=this._getAttributes(l);this.applyTransform(d,c.fields,f),e.add(d),this._buildHierarchy(d,l)}else if(u==="Skeleton"){let d=this._buildSkeleton(l);d&&(this.skeletons[l]=d),this._buildHierarchy(e,l)}else if(u!=="SkelAnimation"){if(u==="Mesh"){let d=this._buildMesh(l,c);d&&(e.add(d),this._buildHierarchy(d,l))}else if(u==="Camera"){let d=this._buildCamera(l);d.name=a;let f=this._getAttributes(l);this.applyTransform(d,c.fields,f),e.add(d),this._buildHierarchy(d,l)}else if(u==="DistantLight"||u==="SphereLight"||u==="RectLight"||u==="DiskLight"){let d=this._buildLight(l,u);d.name=a;let f=this._getAttributes(l);this.applyTransform(d,c.fields,f),e.add(d),this._buildHierarchy(d,l)}else if(u==="Cube"||u==="Sphere"||u==="Cylinder"||u==="Cone"||u==="Capsule"){let d=this._buildGeomPrimitive(l,c,u);d&&(e.add(d),this._buildHierarchy(d,l))}else if(!(u==="Material"||u==="Shader"||u==="GeomSubset")){let d=new At;d.name=a;let f=this._getAttributes(l);this.applyTransform(d,c.fields,f),e.add(d),this._buildHierarchy(d,l)}}}}_getVariantPaths(e){let t=this.specsByPath[e],n=t?.fields?.variantSetChildren,i=[];if(!n||n.length===0)return i;for(let r of n){let o=this.externalVariantSelections[r]||null;if(!o){let a=t.fields.variantSelection;o=a?a[r]:null}if(!o){let a=e+"/{"+r+"=}",l=this.specsByPath[a];l?.fields?.variantChildren&&(o=l.fields.variantChildren[0])}if(o){let a=e+"/{"+r+"="+o+"}";i.push(a)}}return i}_resolveFilePath(e){let t=e;if(t.startsWith("./")&&(t=t.slice(2)),!this.basePath)return t;let n=this.basePath.endsWith("/")?this.basePath:this.basePath+"/";return Ji.resolveURL(t,n)}_resolveReference(e,t={}){if(!e)return null;let n=e.match(/@([^@]+)@(?:<([^>]+)>)?/);if(!n)return null;let i=n[1],r=n[2],o=this._resolveFilePath(i),a={...t,...this.externalVariantSelections},l=this.assets[o];if(!l)return null;if(l.specsByPath){let c=new s(this.manager);c.imageLoader.setCrossOrigin(this.imageLoader.crossOrigin),c.imageLoader.setPriority(this.imageLoader.priority),c.imageLoader.setCache(this.imageLoader.cache);let u=this._getBasePath(o),h=c.compose(l,this.assets,a,u);if(r){let d=r.split("/").pop(),f=null;for(let m of h.children)if(m.name===d){f=m;break}if(f){h.remove(f);let m=new ut;return m.add(f),m}}return h}return l.isGroup||l.isObject3D?l.clone():null}_findSingleMesh(e){for(let t of e.children)if(t.isMesh)return e.remove(t),t;if(e.children.length===1){let t=e.children[0];if(t.children&&t.children.length===1){let n=t.children[0];if(n.isMesh&&!this._hasNonIdentityTransform(t))return t.remove(n),n}}return null}_hasNonIdentityTransform(e){let t=e.position,n=e.rotation,i=e.scale,r=t.x!==0||t.y!==0||t.z!==0,o=n.x!==0||n.y!==0||n.z!==0,a=i.x!==1||i.y!==1||i.z!==1;return r||o||a}_getBasePath(e){let t=e.lastIndexOf("/");return t>=0?e.slice(0,t):""}_getLocalVariantSelections(e){let t={};if(e.variantSelection)for(let n in e.variantSelection)t[n]=e.variantSelection[n];return t}_getReferences(e){let t=[];if(e.fields.references&&e.fields.references.length>0){let n=e.fields.references[0];if(typeof n=="string"){let i=n.matchAll(/@([^@]+)@(?:<([^>]+)>)?/g);for(let r of i)t.push(r[0])}else n.assetPath&&t.push("@"+n.assetPath+"@")}if(t.length===0&&e.fields.payload){let n=e.fields.payload;typeof n=="string"?t.push(n):n.assetPath&&t.push("@"+n.assetPath+"@")}return t}_resolveAttributeValue(e,t=new Set){let n=String(e).replace(/<|>/g,"");if(t.has(n))return;let i=this.specsByPath[n]?.fields;if(!i)return;let r=new Set(t);if(r.add(n),i.connectionPaths)for(let l of i.connectionPaths){let c=this._resolveAttributeValue(l,r);if(c!==void 0)return c}if(i.default!==void 0)return i.default;let{times:o,values:a}=i.timeSamples||{};if(o&&a&&o.length>0){let l=o.indexOf(0);return a[l>=0?l:0]}}_getAttributes(e){let t={};this._collectAttributesFromPath(e,t);let n=e.match(jy);if(n){let i=n[1],r=n[4],o=this._getVariantPaths(i);for(let a of o){if(e.startsWith(a))continue;let l=a+"/"+r;this._collectAttributesFromPath(l,t)}}else{let i=e.split("/");for(let r=1;r<i.length-1;r++){let o=i.slice(0,r+1).join("/"),a=i.slice(r+1).join("/"),l=this._getVariantPaths(o);for(let c of l){let u=c+"/"+a;this._collectAttributesFromPath(u,t)}}}return t}_collectAttributesFromPath(e,t){let n=this.attributesByPrimPath.get(e);if(n)for(let[i,r]of n){let o=this._resolveAttributeValue(e+"."+i);o!==void 0&&(t[i]=o),r.fields?.elementSize!==void 0&&(t[i+":elementSize"]=r.fields.elementSize),i.startsWith("primvars:")&&r.fields?.typeName!==void 0&&(t[i+":typeName"]=r.fields.typeName)}}_buildGeomPrimitive(e,t,n){let i=this._getAttributes(e),r=e.split("/").pop(),o;switch(n){case"Cube":{let u=i.size||2;o=new sn(u,u,u);break}case"Sphere":{let u=i.radius||1;o=new Ur(u,32,16);break}case"Cylinder":{let u=i.height||2,h=i.radius||1;o=new Rn(h,h,u,32);break}case"Cone":{let u=i.height||2,h=i.radius||1;o=new Tr(h,u,32);break}case"Capsule":{let u=i.height||1,h=i.radius||.5;o=new Ar(h,u,16,32);break}}let a=i.axis||"Z";a==="X"?o.rotateZ(-Math.PI/2):a==="Z"&&o.rotateX(Math.PI/2);let l=this._buildMaterial(e,t.fields),c=new ke(o,l);return c.name=r,this.applyTransform(c,t.fields,i),c}_buildMesh(e,t){let n=this._getAttributes(e),i=n["primvars:skel:jointIndices"],r=n["primvars:skel:jointWeights"],o=i&&r&&i.length>0&&r.length>0,a=this._getGeomSubsets(e),l,c;if(a.length>0){l=this._buildGeometryWithSubsets(n,a,o);let f=this._getMaterialPath(e,t.fields);c=a.map(m=>{let _=m.materialPath||f;return this._buildMaterialForPath(_)})}else l=this._buildGeometry(e,n,o),c=this._buildMaterial(e,t.fields);let u=n["primvars:displayColor"];if(u&&u.length>=3){let f=m=>{m.color&&m.color.r===1&&m.color.g===1&&m.color.b===1&&!m.map&&m.color.setRGB(u[0],u[1],u[2],Gt)};Array.isArray(c)?c.forEach(f):f(c)}let h=n["primvars:displayOpacity"];if(h&&h.length===1&&a.length===0){let f=h[0],m=_=>{f<1&&_.opacity===1&&_.transparent===!1&&(_.opacity=f,_.transparent=!0)};Array.isArray(c)?c.forEach(m):m(c)}let d;if(o){d=new yr(l,c);let f=this.specsByPath[e+".skel:skeleton"];f||(f=this.specsByPath[e+".rel skel:skeleton"]);let m=null;f&&(f.fields.targetPaths&&f.fields.targetPaths.length>0?m=f.fields.targetPaths[0]:f.fields.default&&(m=f.fields.default.replace(/<|>/g,"")));let _=n["skel:joints"],p=n["primvars:skel:geomBindTransform"];this.skinnedMeshes.push({mesh:d,skeletonPath:m,path:e,localJoints:_,geomBindTransform:p})}else d=new ke(l,c);return d.name=e.split("/").pop(),this.applyTransform(d,t.fields,n),d}_buildCamera(e){let t=this._getAttributes(e),n=t.projection,i=typeof n=="string"?n.toLowerCase():Nn.projection,r=t.clippingRange||Nn.clippingRange,o=Math.max(Number.EPSILON,this._parseNumber(r[0],Nn.clippingRange[0])),a=Math.max(o+Number.EPSILON,this._parseNumber(r[1],Nn.clippingRange[1])),l=this._parseNumber(t.horizontalAperture,Nn.horizontalAperture),c=this._parseNumber(t.verticalAperture,Nn.verticalAperture),u=this._parseNumber(t.horizontalApertureOffset,Nn.horizontalApertureOffset),h=this._parseNumber(t.verticalApertureOffset,Nn.verticalApertureOffset),d=this._parseNumber(t.focalLength,Nn.focalLength),f=this._parseNumber(t.focusDistance,Nn.focusDistance),m=this._parseNumber(t.fStop,Nn.fStop),_;if(i==="orthographic"){let p=l/10,g=c/10,S=u/10,v=h/10;_=new Yi(S-p*.5,S+p*.5,v+g*.5,v-g*.5,o,a)}else{let p=Math.max(Number.EPSILON,c),g=Math.max(Number.EPSILON,d),S=l/p,v=2*Math.atan(p/(2*g))*180/Math.PI;_=new Ct(v,S,o,a),_.filmGauge=Math.max(l,c),_.filmOffset=u,_.focus=f,_.setFocalLength(g),h!==0&&(_.userData.verticalApertureOffset=h)}return _.userData.fStop=m,_.userData.usdProjection=i,_}_buildLight(e,t){let n=this._getAttributes(e),i=this._parseNumber(n["inputs:intensity"],1),r=n["inputs:color"]||[1,1,1],o=n["inputs:enableColorTemperature"]===!0,a=this._parseNumber(n["inputs:colorTemperature"],6500),l=new Ke(r[0],r[1],r[2]);if(o){let u=this._colorTemperature(a);l.multiply(u)}let c;switch(t){case"DistantLight":c=new Sn(l,i);break;case"SphereLight":{let u=this._parseNumber(n["shaping:cone:angle"],0);if(u>0){let h=u*Math.PI/180,d=this._parseNumber(n["shaping:cone:softness"],0);c=new Vr(l,i,0,h,d)}else c=new qi(l,i);break}case"RectLight":{let u=this._parseNumber(n["inputs:width"],1),h=this._parseNumber(n["inputs:height"],1);c=new ks(l,i,u,h);break}case"DiskLight":{let h=this._parseNumber(n["inputs:radius"],.5)*2;c=new ks(l,i,h,h);break}}return c}_colorTemperature(e){let t=e/100,n,i,r;return t<=66?(n=1,i=.3900815787690196*Math.log(t)-.6318414437886275):(n=1.292936186062745*Math.pow(t-60,-.1332047592),i=1.1298908608952942*Math.pow(t-60,-.0755148492)),t>=66?r=1:t<=19?r=0:r=.543206789110196*Math.log(t-10)-1.19625408914,new Ke(Math.min(Math.max(n,0),1),Math.min(Math.max(i,0),1),Math.min(Math.max(r,0),1))}_parseNumber(e,t){let n=Number(e);return Number.isFinite(n)?n:t}_getGeomSubsets(e){let t=[],n=this.geomSubsetsByMeshPath.get(e);if(!n)return t;for(let i of n){let o=this._getAttributes(i).indices;if(!o||o.length===0)continue;let a=this._getMaterialBindingTarget(i);t.push({name:i.split("/").pop(),indices:o,materialPath:a})}return t}_getMaterialBindingSpec(e){let t="material:binding",n=e+"."+t,i=this.specsByPath[n]||null,r=e.split("/");for(let o=1;o<r.length;o++){let a=r.slice(0,o+1).join("/"),l=r.slice(o+1).join("/"),c=this._getVariantPaths(a);for(let u of c){let h=l?u+"/"+l+"."+t:u+"."+t,d=this.specsByPath[h];d?.fields?.targetPaths?.length>0&&(i=d)}}return i}_getMaterialBindingTarget(e){let t=e.split("/").filter(Boolean),n=null;for(let i=0;i<t.length;i++){let r="/"+t.slice(0,i+1).join("/"),o=this._getMaterialBindingSpec(r);o?.fields?.targetPaths?.length&&(!n||n.fields?.bindMaterialAs!=="strongerThanDescendants")&&(n=o)}return n?.fields.targetPaths[0]||null}_buildGeometry(e,t,n=!1){let i=new Lt,r=t.points;if(!r||r.length===0)return i;let o=t.faceVertexIndices,a=t.faceVertexCounts,l=t["primvars:arnold:polygon_holes"],c=this._buildHoleMap(l),u=o,h=null;if(a&&a.length>0){let x=this._triangulateIndicesWithPattern(o,a,r,c);u=x.indices,h=x.pattern}let d=r;u&&u.length>0&&(d=this._expandAttribute(r,u,3)),i.setAttribute("position",new gt(new Float32Array(d),3));let f=t.normals||t["primvars:normals"],m=t["normals:indices"]||t["primvars:normals:indices"];if(f&&f.length>0){let x=f;if(m&&m.length>0&&h){let b=this._applyTriangulationPattern(m,h);x=this._expandAttribute(f,b,3)}else if(f.length===r.length)u&&u.length>0&&(x=this._expandAttribute(f,u,3));else if(h){let b=this._applyTriangulationPattern(Array.from({length:f.length/3},(w,y)=>y),h);x=this._expandAttribute(f,b,3)}i.setAttribute("normal",new gt(new Float32Array(x),3))}else{let x=this._computeVertexNormals(r,u);i.setAttribute("normal",new gt(new Float32Array(this._expandAttribute(x,u,3)),3))}let{uvs:_,uvIndices:p}=this._findUVPrimvar(t),g=o?o.length:0;if(_&&_.length>0){let x=_;if(p&&p.length>0&&h){let b=this._applyTriangulationPattern(p,h);x=this._expandAttribute(_,b,2)}else if(u&&_.length/2===r.length/3)x=this._expandAttribute(_,u,2);else if(h&&_.length/2===g){let b=this._applyTriangulationPattern(Array.from({length:g},(w,y)=>y),h);x=this._expandAttribute(_,b,2)}i.setAttribute("uv",new gt(new Float32Array(x),2))}let{uvs2:S,uv2Indices:v}=this._findUV2Primvar(t);if(S&&S.length>0){let x=S;if(v&&v.length>0&&h){let b=this._applyTriangulationPattern(v,h);x=this._expandAttribute(S,b,2)}else if(u&&S.length/2===r.length/3)x=this._expandAttribute(S,u,2);else if(h&&S.length/2===g){let b=this._applyTriangulationPattern(Array.from({length:g},(w,y)=>y),h);x=this._expandAttribute(S,b,2)}i.setAttribute("uv1",new gt(new Float32Array(x),2))}if(n){let x=t["primvars:skel:jointIndices"],b=t["primvars:skel:jointWeights"],w=t["primvars:skel:jointIndices:elementSize"]||4;if(x&&b){let y=d.length/3,M,C;u&&u.length>0?(M=this._expandAttribute(x,u,w),C=this._expandAttribute(b,u,w)):(M=x,C=b);let E=new Uint16Array(y*4),R=new Float32Array(y*4);this._selectTopWeights(M,C,w,y,E,R),i.setAttribute("skinIndex",new gt(E,4)),i.setAttribute("skinWeight",new gt(R,4))}}return i}_buildGeometryWithSubsets(e,t,n=!1){let i=new Lt,r=e.points;if(!r||r.length===0)return i;let o=e.faceVertexIndices,a=e.faceVertexCounts;if(!a||a.length===0)return i;let l=e["primvars:arnold:polygon_holes"],c=this._buildHoleMap(l),u=c.holeFaces,h=c.parentToHoles,{uvs:d,uvIndices:f}=this._findUVPrimvar(e),{uvs2:m,uv2Indices:_}=this._findUV2Primvar(e),p=e.normals||e["primvars:normals"],g=e["normals:indices"]||e["primvars:normals:indices"],S=n?e["primvars:skel:jointIndices"]:null,v=n?e["primvars:skel:jointWeights"]:null,x=e["primvars:skel:jointIndices:elementSize"]||4,b=[],w=0;for(let H=0;H<a.length;H++){if(b.push(w),u.has(H))continue;let se=a[H],fe=h.get(H);if(fe&&fe.length>0){let Le=se;for(let V of fe)Le+=a[V];w+=Le-2}else se>=3&&(w+=se-2)}let y=new Int32Array(w).fill(-1);for(let H=0;H<t.length;H++){let se=t[H];for(let fe=0;fe<se.indices.length;fe++){let Le=se.indices[fe];if(Le>=a.length)continue;let V=b[Le],N=a[Le]-2;for(let X=0;X<N;X++)y[V+X]=H}}let M=[];for(let H=0;H<w;H++)M.push({original:H,subset:y[H]});M.sort((H,se)=>H.subset-se.subset);let C=[],E=M.length>0?M[0].subset:-1,R=0;for(let H=0;H<M.length;H++)M[H].subset!==E&&(E>=0&&C.push({start:R*3,count:(H-R)*3,materialIndex:E}),E=M[H].subset,R=H);E>=0&&M.length>R&&C.push({start:R*3,count:(M.length-R)*3,materialIndex:E});for(let H of C)i.addGroup(H.start,H.count,H.materialIndex);let{indices:D,pattern:k}=this._triangulateIndicesWithPattern(o,a,r,c),P=a.reduce((H,se)=>H+se,0),O=d&&!f&&d.length/2===P||m&&!_&&m.length/2===P?this._applyTriangulationPattern(Array.from({length:P},(H,se)=>se),k):null,G=f?this._applyTriangulationPattern(f,k):d&&d.length/2===P?O:null,U=_?this._applyTriangulationPattern(_,k):m&&m.length/2===P?O:null,ee=p&&g&&g.length>0,W=p&&p.length/3===P,K=ee?this._applyTriangulationPattern(g,k):W?this._applyTriangulationPattern(Array.from({length:P},(H,se)=>se),k):null,$=!p&&D.length>0?this._computeVertexNormals(r,D):null,he=w*3,de=new Float32Array(he*3),q=d?new Float32Array(he*2):null,ie=m?new Float32Array(he*2):null,ce=p||$?new Float32Array(he*3):null,Ce=S?new Uint16Array(he*x):null,ue=v?new Float32Array(he*x):null;for(let H=0;H<M.length;H++){let se=M[H].original;for(let fe=0;fe<3;fe++){let Le=se*3+fe,V=H*3+fe,N=D[Le];if(de[V*3]=r[N*3],de[V*3+1]=r[N*3+1],de[V*3+2]=r[N*3+2],q&&d)if(G){let X=G[Le];q[V*2]=d[X*2],q[V*2+1]=d[X*2+1]}else d.length/2===r.length/3&&(q[V*2]=d[N*2],q[V*2+1]=d[N*2+1]);if(ie&&m)if(U){let X=U[Le];ie[V*2]=m[X*2],ie[V*2+1]=m[X*2+1]}else m.length/2===r.length/3&&(ie[V*2]=m[N*2],ie[V*2+1]=m[N*2+1]);if(ce)if(p&&K){let X=K[Le];ce[V*3]=p[X*3],ce[V*3+1]=p[X*3+1],ce[V*3+2]=p[X*3+2]}else p&&p.length===r.length?(ce[V*3]=p[N*3],ce[V*3+1]=p[N*3+1],ce[V*3+2]=p[N*3+2]):$&&(ce[V*3]=$[N*3],ce[V*3+1]=$[N*3+1],ce[V*3+2]=$[N*3+2]);if(Ce&&ue&&S&&v)for(let X=0;X<x;X++)Ce[V*x+X]=S[N*x+X]||0,ue[V*x+X]=v[N*x+X]||0}}if(i.setAttribute("position",new gt(de,3)),q&&i.setAttribute("uv",new gt(q,2)),ie&&i.setAttribute("uv1",new gt(ie,2)),i.setAttribute("normal",new gt(ce,3)),Ce&&ue){let H=new Uint16Array(he*4),se=new Float32Array(he*4);this._selectTopWeights(Ce,ue,x,he,H,se),i.setAttribute("skinIndex",new gt(H,4)),i.setAttribute("skinWeight",new gt(se,4))}return i}_selectTopWeights(e,t,n,i,r,o){if(n<=4){for(let l=0;l<i;l++)for(let c=0;c<4;c++)c<n?(r[l*4+c]=e[l*n+c]||0,o[l*4+c]=t[l*n+c]||0):(r[l*4+c]=0,o[l*4+c]=0);return}let a=new Uint32Array(n);for(let l=0;l<i;l++){let c=l*n;for(let h=0;h<n;h++)a[h]=h;for(let h=0;h<4;h++){let d=h,f=t[c+a[h]]||0;for(let m=h+1;m<n;m++){let _=t[c+a[m]]||0;_>f&&(f=_,d=m)}if(d!==h){let m=a[h];a[h]=a[d],a[d]=m}}let u=0;for(let h=0;h<4;h++)u+=t[c+a[h]]||0;for(let h=0;h<4;h++){let d=a[h];u>0?(r[l*4+h]=e[c+d]||0,o[l*4+h]=(t[c+d]||0)/u):(r[l*4+h]=0,o[l*4+h]=0)}}}_findUVPrimvar(e){for(let i in e){if(!i.startsWith("primvars:")||i.endsWith(":typeName")||i.endsWith(":elementSize")||i.endsWith(":indices")||i.includes("skel:"))continue;let r=e[i+":typeName"];if(r&&r.includes("texCoord"))return{uvs:e[i],uvIndices:e[i+":indices"]}}let t=e["primvars:st"]||e["primvars:UVMap"],n=e["primvars:st:indices"];return{uvs:t,uvIndices:n}}_findUV2Primvar(e){let t=e["primvars:st1"],n=e["primvars:st1:indices"];return{uvs2:t,uv2Indices:n}}_buildHoleMap(e){if(!e||e.length===0)return{parentToHoles:new Map,holeFaces:new Set};let t=new Map,n=new Set;for(let i=0;i<e.length;i+=2){let r=e[i],o=e[i+1];n.add(r),t.has(o)||t.set(o,[]),t.get(o).push(r)}return{parentToHoles:t,holeFaces:n}}_triangulateIndicesWithPattern(e,t,n=null,i=null){let r=[],o=[],a=[],l=0;for(let d=0;d<t.length;d++)a.push(l),l+=t[d];let c=i?.parentToHoles||new Map,u=i?.holeFaces||new Set,h=0;for(let d=0;d<t.length;d++){let f=t[d];if(u.has(d)){h+=f;continue}let m=c.get(d);if(m&&m.length>0&&n&&n.length>0){let _=new Map,p=[];for(let v=0;v<f;v++){let x=e[h+v];p.push(x),_.set(x,h+v)}let g=[];for(let v of m){let x=a[v],b=t[v],w=[];for(let y=0;y<b;y++){let M=e[x+y];w.push(M),_.set(M,x+y)}g.push(w)}let S=this._triangulateNGonWithHoles(p,g,n);for(let v of S)r.push(v[0],v[1],v[2]),o.push(_.get(v[0]),_.get(v[1]),_.get(v[2]))}else if(f===3)r.push(e[h],e[h+1],e[h+2]),o.push(h,h+1,h+2);else if(f===4)r.push(e[h],e[h+1],e[h+2],e[h],e[h+2],e[h+3]),o.push(h,h+1,h+2,h,h+2,h+3);else if(f>4)if(n&&n.length>0){let _=[];for(let g=0;g<f;g++)_.push(e[h+g]);let p=this._triangulateNGon(_,n);for(let g of p)r.push(g[0],g[1],g[2]),o.push(h+_.indexOf(g[0]),h+_.indexOf(g[1]),h+_.indexOf(g[2]))}else for(let _=1;_<f-1;_++)r.push(e[h],e[h+_],e[h+_+1]),o.push(h,h+_,h+_+1);h+=f}return{indices:r,pattern:o}}_applyTriangulationPattern(e,t){let n=[];for(let i=0;i<t.length;i++)n.push(e[t[i]]);return n}_triangulateNGon(e,t){let n=[],i=[];for(let u of e)i.push(new L(t[u*3],t[u*3+1],t[u*3+2]));let r=new L;for(let u=0;u<i.length;u++){let h=i[u],d=i[(u+1)%i.length];r.x+=(h.y-d.y)*(h.z+d.z),r.y+=(h.z-d.z)*(h.x+d.x),r.z+=(h.x-d.x)*(h.y+d.y)}r.normalize();let o=new L,a=new L;Math.abs(r.y)>.9?o.set(1,0,0):o.set(0,1,0),a.crossVectors(r,o).normalize(),o.crossVectors(a,r).normalize();for(let u of i)n.push(new pe(u.dot(o),u.dot(a)));let l=cn.triangulateShape(n,[]),c=[];for(let u of l)c.push([e[u[0]],e[u[1]],e[u[2]]]);return c}_triangulateNGonWithHoles(e,t,n){let i=[];for(let f of e)i.push(new L(n[f*3],n[f*3+1],n[f*3+2]));let r=new L;for(let f=0;f<i.length;f++){let m=i[f],_=i[(f+1)%i.length];r.x+=(m.y-_.y)*(m.z+_.z),r.y+=(m.z-_.z)*(m.x+_.x),r.z+=(m.x-_.x)*(m.y+_.y)}r.normalize();let o=new L,a=new L;Math.abs(r.y)>.9?o.set(1,0,0):o.set(0,1,0),a.crossVectors(r,o).normalize(),o.crossVectors(a,r).normalize();let l=[];for(let f of i)l.push(new pe(f.dot(o),f.dot(a)));let c=[];for(let f of t){let m=[];for(let _ of f){let p=new L(n[_*3],n[_*3+1],n[_*3+2]);m.push(new pe(p.dot(o),p.dot(a)))}c.push(m)}let u=[...e];for(let f of t)u.push(...f);let h=cn.triangulateShape(l,c),d=[];for(let f of h)d.push([u[f[0]],u[f[1]],u[f[2]]]);return d}_triangulateIndices(e,t){let n=[],i=0;for(let r=0;r<t.length;r++){let o=t[r];if(o===3)n.push(e[i],e[i+1],e[i+2]);else if(o===4)n.push(e[i],e[i+1],e[i+2],e[i],e[i+2],e[i+3]);else if(o>4)for(let a=1;a<o-1;a++)n.push(e[i],e[i+a],e[i+a+1]);i+=o}return n}_expandAttribute(e,t,n){let i=new Array(t.length*n);for(let r=0;r<t.length;r++){let o=t[r];for(let a=0;a<n;a++)i[r*n+a]=e[o*n+a]}return i}_computeVertexNormals(e,t){let n=e.length/3,i=new Float32Array(n*3);for(let r=0;r<t.length;r+=3){let o=t[r],a=t[r+1],l=t[r+2],c=e[o*3],u=e[o*3+1],h=e[o*3+2],d=e[a*3],f=e[a*3+1],m=e[a*3+2],_=e[l*3],p=e[l*3+1],g=e[l*3+2],S=d-c,v=f-u,x=m-h,b=_-c,w=p-u,y=g-h,M=v*y-x*w,C=x*b-S*y,E=S*w-v*b;i[o*3]+=M,i[o*3+1]+=C,i[o*3+2]+=E,i[a*3]+=M,i[a*3+1]+=C,i[a*3+2]+=E,i[l*3]+=M,i[l*3+1]+=C,i[l*3+2]+=E}for(let r=0;r<n;r++){let o=i[r*3],a=i[r*3+1],l=i[r*3+2],c=Math.sqrt(o*o+a*a+l*l);c>0&&(i[r*3]/=c,i[r*3+1]/=c,i[r*3+2]/=c)}return i}_getMaterialPath(e,t){let n=null,i=t["material:binding"];return i&&(n=Array.isArray(i)?i[0]:i),n||(n=this._getMaterialBindingTarget(e)),n}_buildMaterial(e,t){let n=new nn,i=null,r=t["material:binding"];if(r&&(i=Array.isArray(r)?r[0]:r),i||(i=this._getMaterialBindingTarget(e)),!i){let o=[],a=e+"/";for(let l in this.specsByPath){if(!l.startsWith(a)||!l.endsWith(".material:binding"))continue;let c=this.specsByPath[l];if(!c)continue;let u=c.fields.targetPaths;u&&u.length>0&&o.push(u[0])}o.length>0&&(i=this._pickBestMaterial(o))}if(!i){let a="/"+e.split("/")[1],l=this.materialsByRoot.get(a);if(l){for(let c of l)if(c.startsWith(a+"/Looks/")||c.startsWith(a+"/Materials/")){i=c;break}}}return i&&this._applyMaterial(n,i),n}_buildMaterialForPath(e){let t=new nn;return e&&this._applyMaterial(t,e),t}_applyMaterialBinding(e,t){let n=this._getMaterialBindingTarget(t);if(!n)return;n=String(n).replace(/^<|>$/g,"");let i=new nn;this._applyMaterial(i,n),e.material=i}_pickBestMaterial(e){for(let t of e){let n=this.shadersByMaterialPath.get(t);if(n)for(let i of n){let r=this._getAttributes(i);if(r["info:id"]==="UsdUVTexture"&&r["inputs:file"])return t}}return e[0]}_applyMaterial(e,t){if(!this.specsByPath[t])return;let i=this.shadersByMaterialPath.get(t);if(i)for(let r of i){let o=this.specsByPath[r];if(!o)continue;let l=this._getAttributes(r)["info:id"]||o.fields["info:id"];l==="UsdPreviewSurface"||l==="ND_UsdPreviewSurface_surfaceshader"?this._applyPreviewSurface(e,r):l==="arnold:openpbr_surface"&&this._applyOpenPBRSurface(e,r)}}_applyTextureOrValue(e,t,n,i,r,o,a,l){let c=t+"."+i,u=this.specsByPath[c];if(u&&u.fields.connectionPaths&&u.fields.connectionPaths.length>0){let h=l===this._getTextureFromOpenPBRConnection?u.fields.connectionPaths:[u.fields.connectionPaths[0]];for(let d of h){let f=l.call(this,d);if(f)return f.colorSpace=o,e[r]=f,!0}}return n[i]!==void 0&&a&&a(n[i]),!1}_applyPreviewSurface(e,t){let n=this._getAttributes(t),i=(h,d,f,m)=>this._applyTextureOrValue(e,t,n,h,d,f,m,this._getTextureFromConnection),r=h=>{let d=t+"."+h;return this.specsByPath[d]};if(i("inputs:diffuseColor","map",wt,h=>{Array.isArray(h)&&h.length>=3&&e.color.setRGB(h[0],h[1],h[2],Gt)}),e.map&&e.map.userData.scale){let h=e.map.userData.scale;Array.isArray(h)&&h.length>=3&&e.color.setRGB(h[0],h[1],h[2],Gt)}if(i("inputs:emissiveColor","emissiveMap",wt,h=>{Array.isArray(h)&&h.length>=3&&e.emissive.setRGB(h[0],h[1],h[2],Gt)}),e.emissiveMap)if(e.emissiveMap.userData.scale){let h=e.emissiveMap.userData.scale;Array.isArray(h)&&h.length>=3&&e.emissive.setRGB(h[0],h[1],h[2],Gt)}else e.emissive.set(16777215);if(i("inputs:normal","normalMap",qt,null),e.normalMap&&e.normalMap.userData.scale){let h=e.normalMap.userData.scale;e.normalScale=new pe(h[0],h[1])}if(i("inputs:roughness","roughnessMap",qt,h=>{e.roughness=h})&&(e.roughness=1),i("inputs:metallic","metalnessMap",qt,h=>{e.metalness=h})&&(e.metalness=1),i("inputs:occlusion","aoMap",qt,null),n["inputs:ior"]!==void 0&&(e.ior=n["inputs:ior"]),i("inputs:specularColor","specularColorMap",wt,h=>{Array.isArray(h)&&h.length>=3&&e.specularColor.setRGB(h[0],h[1],h[2],Gt)}),e.specularColorMap&&e.specularColorMap.userData.scale){let h=e.specularColorMap.userData.scale;Array.isArray(h)&&h.length>=3&&e.specularColor.setRGB(h[0],h[1],h[2],Gt)}n["inputs:clearcoat"]!==void 0&&(e.clearcoat=n["inputs:clearcoat"]),n["inputs:clearcoatRoughness"]!==void 0&&(e.clearcoatRoughness=n["inputs:clearcoatRoughness"]);let l=n["inputs:opacityThreshold"]!==void 0?n["inputs:opacityThreshold"]:0;if(r("inputs:opacity")?.fields?.connectionPaths?.length>0)l>0?(e.alphaTest=l,e.transparent=!1):e.transparent=!0;else{let h=n["inputs:opacity"]!==void 0?n["inputs:opacity"]:1;h<1&&(e.transparent=!0,e.opacity=h)}}_applyOpenPBRSurface(e,t){let n=this._getAttributes(t),i=(_,p,g,S)=>this._applyTextureOrValue(e,t,n,_,p,g,S,this._getTextureFromOpenPBRConnection);if(i("inputs:base_color","map",wt,_=>{Array.isArray(_)&&_.length>=3&&e.color.setRGB(_[0],_[1],_[2],Gt)}),e.map&&e.map.userData.scale){let _=e.map.userData.scale;Array.isArray(_)&&_.length>=3&&e.color.setRGB(_[0],_[1],_[2],Gt)}i("inputs:base_metalness","metalnessMap",qt,_=>{typeof _=="number"&&(e.metalness=_)}),i("inputs:specular_roughness","roughnessMap",qt,_=>{typeof _=="number"&&(e.roughness=_)});let r=i("inputs:emission_color","emissiveMap",wt,_=>{Array.isArray(_)&&_.length>=3&&e.emissive.setRGB(_[0],_[1],_[2],Gt)}),o=n["inputs:emission_luminance"];o!==void 0&&o>0&&(r?e.emissiveIntensity=o:e.emissive.multiplyScalar(o));let a=n["inputs:transmission_weight"];if(a!==void 0&&a>0){e.transmission=a;let _=n["inputs:transmission_depth"];_!==void 0&&(e.thickness=_);let p=n["inputs:transmission_color"];p!==void 0&&Array.isArray(p)&&(e.attenuationColor.setRGB(p[0],p[1],p[2],Gt),e.attenuationDistance=_||1)}let l=n["inputs:geometry_opacity"];l!==void 0&&l<1&&(e.opacity=l,e.transparent=!0);let c=n["inputs:specular_ior"];c!==void 0&&(e.ior=c);let u=n["inputs:coat_weight"];if(u!==void 0&&u>0){e.clearcoat=u;let _=n["inputs:coat_roughness"];_!==void 0&&(e.clearcoatRoughness=_)}let h=n["inputs:thin_film_weight"];if(h!==void 0&&h>0){e.iridescence=h;let _=n["inputs:thin_film_ior"];_!==void 0&&(e.iridescenceIOR=_);let p=n["inputs:thin_film_thickness"];if(p!==void 0){let g=p*1e3;e.iridescenceThicknessMinimum=g,e.iridescenceThicknessMaximum=g}}let d=n["inputs:specular_weight"];d!==void 0&&(e.specularIntensity=d);let f=n["inputs:specular_color"];f!==void 0&&Array.isArray(f)&&e.specularColor.setRGB(f[0],f[1],f[2],Gt);let m=n["inputs:specular_roughness_anisotropy"];m!==void 0&&m>0&&(e.anisotropy=m),i("inputs:geometry_normal","normalMap",qt,null)}_getTextureFromOpenPBRConnection(e){let t=e.replace(/<|>/g,""),n=t.split(".")[0],i=this.specsByPath[n];if(!i)return null;let r=this._getAttributes(n),o=r["info:id"]||i.fields["info:id"];if(i.fields.typeName==="NodeGraph"){let c=t.split(".")[1],u=n+"."+c,h=this.specsByPath[u];return h?.fields?.connectionPaths?.length>0?this._getTextureFromOpenPBRConnection(h.fields.connectionPaths[0]):null}if(o==="arnold:image"){let c=r["inputs:filename"];return c?this._loadTextureFromPath(c):null}if(o&&o.startsWith("ND_image_")){let c=r["inputs:file"];return c?this._loadTextureFromPath(c):null}if(o==="MayaND_fileTexture_color4"){let c=n+".inputs:inColor",u=this.specsByPath[c];return u?.fields?.connectionPaths?.length>0?this._getTextureFromOpenPBRConnection(u.fields.connectionPaths[0]):null}if(o&&o.startsWith("ND_convert_")){let c=n+".inputs:in",u=this.specsByPath[c];return u?.fields?.connectionPaths?.length>0?this._getTextureFromOpenPBRConnection(u.fields.connectionPaths[0]):null}if(o==="arnold:bump2d"){let c=n+".inputs:bump_map",u=this.specsByPath[c];return u?.fields?.connectionPaths?.length>0?this._getTextureFromOpenPBRConnection(u.fields.connectionPaths[0]):null}if(o==="arnold:color_correct"){let c=n+".inputs:input",u=this.specsByPath[c];return u?.fields?.connectionPaths?.length>0?this._getTextureFromOpenPBRConnection(u.fields.connectionPaths[0]):null}let l=n.substring(0,n.lastIndexOf("/"));if(l){let c=this.specsByPath[l];if(c){let u=this._getAttributes(l);if((u["info:id"]||c.fields["info:id"])==="arnold:image"){let d=u["inputs:filename"];if(d)return this._loadTextureFromPath(d)}}}return null}_loadTextureFromPath(e){if(!e)return null;if(this.textureCache[e])return this.textureCache[e];let t=this._loadTexture(e,null,null);return t&&(this.textureCache[e]=t),t}_getTextureFromConnection(e){let t=e.split(".")[0],n=this.specsByPath[t];if(!n)return null;let i=this._getAttributes(t);if((i["info:id"]||n.fields["info:id"])!=="UsdUVTexture")return null;let o=i["inputs:file"];if(!o)return null;let a=null,l=0,c=t+".inputs:st",u=this.specsByPath[c];if(u?.fields?.connectionPaths?.length>0){let p=u.fields.connectionPaths[0].replace(/<|>/g,"").split(".")[0],g=this.specsByPath[p];if(g){let S=this._getAttributes(p),v=S["info:id"]||g.fields["info:id"];if(v==="UsdTransform2d"){a=S;let x=p+".inputs:in",b=this.specsByPath[x];if(b?.fields?.connectionPaths?.length>0){let y=b.fields.connectionPaths[0].replace(/<|>/g,"").split(".")[0],C=this._getAttributes(y)["inputs:varname"];C==="st1"?l=1:C==="st2"&&(l=2)}}else if(v==="UsdPrimvarReader_float2"){let x=S["inputs:varname"];x==="st1"?l=1:x==="st2"&&(l=2)}}}let h=i["inputs:scale"],d=i["inputs:bias"],f=o;if(h&&(f+=":s"+h.join(",")),d&&(f+=":b"+d.join(",")),this.textureCache[f])return this.textureCache[f];let m=this._loadTexture(o,i,a);return m&&(h&&(m.userData.scale=h),d&&(m.userData.bias=d),l!==0&&(m.channel=l),this.textureCache[f]=m),m}_applyTextureTransforms(e,t){if(!t)return;let n=t["inputs:scale"];n&&Array.isArray(n)&&n.length>=2&&e.repeat.set(n[0],n[1]);let i=t["inputs:translation"];i&&Array.isArray(i)&&i.length>=2&&e.offset.set(i[0],i[1]);let r=t["inputs:rotation"];typeof r=="number"&&(e.rotation=r*Math.PI/180)}_loadTexture(e,t,n){let i=e;i.startsWith("@")&&(i=i.slice(1)),i.endsWith("@")&&(i=i.slice(0,-1));let r=this._resolveFilePath(i),o=this.assets[r];if(o||(o=this.assets[i]),!o){let a=i.split("/").pop();for(let l in this.assets)if(l.endsWith(a)||l.endsWith("/"+a))return this._createTextureFromData(this.assets[l],t,n);return this.basePath?this._createTextureFromData(r,t,n):this.manager?this._createTextureFromData(a,t,n):(console.warn("USDLoader: Texture not found:",i),null)}return this._createTextureFromData(o,t,n)}_createTextureFromData(e,t,n){if(!e)return null;let i=this,r=new Nt,o;if(typeof e=="string")o=e;else if(e instanceof Uint8Array||e instanceof ArrayBuffer){let a=new Blob([e]);o=URL.createObjectURL(a)}else return null;return this.texturePromises.push(new Promise(a=>{this.imageLoader.load(o,function(l){r.image=l,t&&(r.wrapS=i._getWrapMode(t["inputs:wrapS"]),r.wrapT=i._getWrapMode(t["inputs:wrapT"])),i._applyTextureTransforms(r,n),r.needsUpdate=!0,typeof e!="string"&&URL.revokeObjectURL(o),a()},void 0,function(){console.warn("USDLoader: Failed to load texture:",o),typeof e!="string"&&URL.revokeObjectURL(o),a()})})),r}_getWrapMode(e){return e==="repeat"?Gi:e==="mirror"?ws:e==="clamp"?_n:Gi}_buildSkeleton(e){let t=this._getAttributes(e),n=t.joints;if(!n||n.length===0)return null;let i=t.bindTransforms,r=t.restTransforms,o=this._flattenMatrixArray(i,n.length),a=this._flattenMatrixArray(r,n.length),l=[],c={},u=[];for(let m=0;m<n.length;m++){let _=n[m],p=_.split("/").pop(),g=new Ds;if(g.name=p,l.push(g),c[_]={bone:g,index:m},o&&o.length>=(m+1)*16){let S=new ze,v=o.slice(m*16,(m+1)*16);S.set(v[0],v[4],v[8],v[12],v[1],v[5],v[9],v[13],v[2],v[6],v[10],v[14],v[3],v[7],v[11],v[15]);let x=S.clone().invert();u.push(x)}else u.push(new ze)}for(let m=0;m<n.length;m++){let p=n[m].split("/");if(p.length>1){let g=p.slice(0,-1).join("/"),S=c[g];S&&S.bone.add(l[m])}}if(a&&a.length>=n.length*16)for(let m=0;m<n.length;m++){let _=new ze,p=a.slice(m*16,(m+1)*16);_.set(p[0],p[4],p[8],p[12],p[1],p[5],p[9],p[13],p[2],p[6],p[10],p[14],p[3],p[7],p[11],p[15]),_.decompose(l[m].position,l[m].quaternion,l[m].scale)}let h=l.filter(m=>!m.parent||!m.parent.isBone),d=this.specsByPath[e+".skel:animationSource"],f=null;return d&&d.fields.targetPaths&&d.fields.targetPaths.length>0&&(f=d.fields.targetPaths[0]),{skeleton:new vr(l,u),joints:n,rootBones:h,animationPath:f,path:e}}_bindSkeletons(){for(let e of this.skinnedMeshes){let{mesh:t,skeletonPath:n,localJoints:i,geomBindTransform:r}=e,o=null;if(n&&this.skeletons[n]&&(o=this.skeletons[n]),!o){for(let h in this.skeletons)if(n&&(n.includes(h)||h.includes(n))){o=this.skeletons[h];break}}if(!o){let h=Object.keys(this.skeletons);h.length>0&&(o=this.skeletons[h[0]])}if(!o){console.warn("USDComposer: No skeleton found for skinned mesh",t.name);continue}let{skeleton:a,rootBones:l,joints:c}=o;if(i&&i.length>0){let h=t.geometry.attributes.skinIndex;if(h){let d=[];for(let m=0;m<i.length;m++){let _=i[m],p=c.indexOf(_);d[m]=p>=0?p:0}let f=h.array;for(let m=0;m<f.length;m++){let _=f[m];_<d.length&&(f[m]=d[_])}}}for(let h of l)t.add(h);let u=new ze;if(r&&r.length===16){let h=r;u.set(h[0],h[4],h[8],h[12],h[1],h[5],h[9],h[13],h[2],h[6],h[10],h[14],h[3],h[7],h[11],h[15])}t.bind(a,u)}}_buildAnimations(){let e=[];for(let n in this.specsByPath){let i=this.specsByPath[n];if(i.specType!==ns.Prim||i.fields.typeName!=="SkelAnimation")continue;let r=this._buildAnimationClip(n);r&&e.push(r)}let t=this._buildTransformAnimations();return t.length>0&&e.push(new Bs("TransformAnimation",-1,t)),e}_buildTransformAnimations(){let e=[];for(let t in this.specsByPath){let n=this.specsByPath[t];if(n.specType!==ns.Prim)continue;let i=n.fields?.typeName;if(i!=="Xform"&&i!=="Scope"&&i!=="Mesh")continue;let r=t.split("/").pop(),o=t+".xformOp:orient",a=this.specsByPath[o];if(a?.fields?.timeSamples){let{times:_,values:p}=a.fields.timeSamples,g=[],S=[];for(let v=0;v<_.length;v++){g.push(_[v]/this.fps);let x=p[v];S.push(x[0],x[1],x[2],x[3])}g.length>0&&e.push(new Cn(r+".quaternion",new Float32Array(g),new Float32Array(S)))}let l=t+".xformOp:rotateXYZ",c=this.specsByPath[l];if(c?.fields?.timeSamples){let{times:_,values:p}=c.fields.timeSamples,g=[],S=[],v=new Ut,x=new st;for(let b=0;b<_.length;b++){g.push(_[b]/this.fps);let w=p[b];v.set(w[0]*Math.PI/180,w[1]*Math.PI/180,w[2]*Math.PI/180,"ZYX"),x.setFromEuler(v),S.push(x.x,x.y,x.z,x.w)}g.length>0&&e.push(new Cn(r+".quaternion",new Float32Array(g),new Float32Array(S)))}let u=t+".xformOp:translate",h=this.specsByPath[u];if(h?.fields?.timeSamples){let{times:_,values:p}=h.fields.timeSamples,g=[],S=[];for(let v=0;v<_.length;v++){g.push(_[v]/this.fps);let x=p[v];S.push(x[0],x[1],x[2])}g.length>0&&e.push(new vn(r+".position",new Float32Array(g),new Float32Array(S)))}let d=t+".xformOp:scale",f=this.specsByPath[d];if(f?.fields?.timeSamples){let{times:_,values:p}=f.fields.timeSamples,g=[],S=[];for(let v=0;v<_.length;v++){g.push(_[v]/this.fps);let x=p[v];S.push(x[0],x[1],x[2])}g.length>0&&e.push(new vn(r+".scale",new Float32Array(g),new Float32Array(S)))}let m=n.fields?.properties||[];for(let _ of m){if(!_.startsWith("xformOp:transform"))continue;let p=t+"."+_,g=this.specsByPath[p];if(!g?.fields?.timeSamples)continue;let{times:S,values:v}=g.fields.timeSamples,x=[],b=[],w=[],y=[],M=[],C=[],E=new ze,R=new L,D=new st,k=new L;for(let P=0;P<S.length;P++){let O=v[P];if(!O||O.length<16)continue;let G=S[P]/this.fps;E.set(O[0],O[4],O[8],O[12],O[1],O[5],O[9],O[13],O[2],O[6],O[10],O[14],O[3],O[7],O[11],O[15]),E.decompose(R,D,k),x.push(G),b.push(R.x,R.y,R.z),w.push(G),y.push(D.x,D.y,D.z,D.w),M.push(G),C.push(k.x,k.y,k.z)}x.length>0&&(e.push(new vn(r+".position",new Float32Array(x),new Float32Array(b))),e.push(new Cn(r+".quaternion",new Float32Array(w),new Float32Array(y))),e.push(new vn(r+".scale",new Float32Array(M),new Float32Array(C))));break}}return e}_buildAnimationClip(e){let n=this._getAttributes(e).joints;if(!n||n.length===0)return null;let i=[],r=this._getTimeSampledAttribute(e,"rotations");if(r&&r.times&&r.values){let{times:c,values:u}=r;for(let h=0;h<n.length;h++){let d=n[h].split("/").pop(),f=[],m=[];for(let _=0;_<c.length;_++){let p=u[_];if(!p||p.length<(h+1)*4)continue;f.push(c[_]/this.fps);let g=p[h*4+0],S=p[h*4+1],v=p[h*4+2],x=p[h*4+3];m.push(g,S,v,x)}f.length>0&&i.push(new Cn(d+".quaternion",new Float32Array(f),new Float32Array(m)))}}let o=this._getTimeSampledAttribute(e,"translations");if(o&&o.times&&o.values){let{times:c,values:u}=o;for(let h=0;h<n.length;h++){let d=n[h].split("/").pop(),f=[],m=[];for(let _=0;_<c.length;_++){let p=u[_];!p||p.length<(h+1)*3||(f.push(c[_]/this.fps),m.push(p[h*3+0],p[h*3+1],p[h*3+2]))}f.length>0&&i.push(new vn(d+".position",new Float32Array(f),new Float32Array(m)))}}let a=this._getTimeSampledAttribute(e,"scales");if(a&&a.times&&a.values){let{times:c,values:u}=a;for(let h=0;h<n.length;h++){let d=n[h].split("/").pop(),f=[],m=[];for(let _=0;_<c.length;_++){let p=u[_];!p||p.length<(h+1)*3||(f.push(c[_]/this.fps),m.push(p[h*3+0],p[h*3+1],p[h*3+2]))}f.length>0&&i.push(new vn(d+".scale",new Float32Array(f),new Float32Array(m)))}}if(i.length===0)return null;let l=e.split("/").pop();return new Bs(l,-1,i)}_getTimeSampledAttribute(e,t){let n=e+"."+t,i=this.specsByPath[n];if(i&&i.fields.timeSamples){let r=i.fields.timeSamples;if(r.times&&r.values)return r}return null}_flattenMatrixArray(e,t){if(!e||e.length===0)return null;if(typeof e[0]=="number")return e;let n=[];for(let i=0;i<t;i++)for(let r=0;r<4;r++){let o=e[i*4+r];o&&o.length===4?n.push(o[0],o[1],o[2],o[3]):n.push(r===0?1:0,r===1?1:0,r===2?1:0,r===3?1:0)}return n}};var yl=class extends Ti{constructor(e){super(e)}load(e,t,n,i){let r=this,o=r.path===""?Ji.extractUrlBase(e):r.path,a=new Br(r.manager);a.setPriority(r.priority),a.setCache(r.cache),a.setPath(r.path),a.setResponseType("arraybuffer"),a.setRequestHeader(r.requestHeader),a.setWithCredentials(r.withCredentials),a.load(e,function(l){try{r.parse(l,o,t,i)}catch(c){i?i(c):console.error(c),r.manager.itemError(e)}},n,i)}parse(e,t="",n,i){let r=new _l,o=new xl,a=new TextDecoder;function l(v){return v instanceof ArrayBuffer?v:v.byteOffset===0&&v.byteLength===v.buffer.byteLength?v.buffer:v.buffer.slice(v.byteOffset,v.byteOffset+v.byteLength)}function c(v){let x=v.lastIndexOf(".");return x<0||v.lastIndexOf("/")>x?"":v.slice(x+1).toLowerCase()}function u(v){let x={};for(let b in v){let w=v[b],y=c(b);if(y==="png"||y==="jpg"||y==="jpeg"||y==="avif"){x[b]=w;continue}y!=="usd"&&y!=="usda"&&y!=="usdc"||(h(w)?x[b]=o.parseData(l(w)):x[b]=r.parseData(a.decode(w)))}return x}function h(v){let x=new Uint8Array([80,88,82,45,85,83,68,67]),b=v instanceof Uint8Array?v:new Uint8Array(v);if(b.byteLength<x.length)return!1;for(let w=0;w<x.length;w++)if(b[w]!==x[w])return!1;return!0}function d(v){let x=Object.keys(v);if(x.length<1)return{file:void 0,filename:"",basePath:""};let b=x[0],w=c(b),y=!1,M=b.lastIndexOf("/"),C=M>=0?b.slice(0,M):"";if(w==="usda")return{file:v[b],filename:b,basePath:C};if(w==="usdc")y=!0;else if(w==="usd")if(h(v[b]))y=!0;else return{file:v[b],filename:b,basePath:C};return y?{file:v[b],filename:b,basePath:C}:{file:void 0,filename:"",basePath:""}}let f=this,m=(v,x)=>(n&&Promise.all(v.texturePromises).then(()=>n(x)).catch(b=>i?i(b):console.error(b)),x);if(typeof e=="string"){let v=new Zn(f.manager);v.imageLoader.setCrossOrigin(f.crossOrigin),v.imageLoader.setPriority(f.priority),v.imageLoader.setCache(f.cache);let x=r.parseData(e);return m(v,v.compose(x,{},{},t))}if(h(e)){let v=new Zn(f.manager);v.imageLoader.setCrossOrigin(f.crossOrigin),v.imageLoader.setPriority(f.priority),v.imageLoader.setCache(f.cache);let x=o.parseData(l(e));return m(v,v.compose(x,{},{},t))}let _=new Uint8Array(e);if(_[0]===80&&_[1]===75){let v=pl(_),x=u(v),{file:b,filename:w,basePath:y}=d(v);if(!b)throw new Error("THREE.USDLoader: Invalid USDZ package. The first ZIP entry must be a USD layer (.usd/.usda/.usdc).");let M=new Zn(f.manager);M.imageLoader.setCrossOrigin(f.crossOrigin),M.imageLoader.setPriority(f.priority),M.imageLoader.setCache(f.cache);let C=x[w];if(!C)throw new Error('THREE.USDLoader: Failed to parse root layer "'+w+'".');return m(M,M.compose(C,x,{},y))}let p=new Zn(f.manager);p.imageLoader.setCrossOrigin(f.crossOrigin),p.imageLoader.setPriority(f.priority),p.imageLoader.setCache(f.cache);let g=a.decode(_),S=r.parseData(g);return m(p,p.compose(S,{},{},t))}};var Qy=Zn.prototype.applyTransform;Zn.prototype.applyTransform=function(s,e,t={}){let n={...e,...t},i=n.xformOpOrder??[];if(!i.some(a=>a.split(":").length>2&&!a.endsWith("xformOp:translate:pivot")))return Qy.call(this,s,e,t);let r=new ze,o=new ze;for(let a of i){let l=a.startsWith("!invert!"),c=l?a.slice(8):a,u=n[c];if(u===void 0)continue;let h=d=>d*Math.PI/180;switch(c.split(":")[1]){case"translate":o.makeTranslation(u[0],u[1],u[2]);break;case"scale":typeof u=="number"?o.makeScale(u,u,u):o.makeScale(u[0],u[1],u[2]);break;case"rotateX":o.makeRotationX(h(u));break;case"rotateY":o.makeRotationY(h(u));break;case"rotateZ":o.makeRotationZ(h(u));break;case"rotateXYZ":o.makeRotationFromEuler(new Ut(h(u[0]),h(u[1]),h(u[2]),"ZYX"));break;case"orient":o.makeRotationFromQuaternion(new st(u[0],u[1],u[2],u[3]));break;case"transform":o.set(u[0],u[4],u[8],u[12],u[1],u[5],u[9],u[13],u[2],u[6],u[10],u[14],u[3],u[7],u[11],u[15]);break;default:continue}l&&o.invert(),r.multiply(o)}s.matrix.copy(r),s.matrix.decompose(s.position,s.quaternion,s.scale)};var ev={"iphone-18-pro":{body:"UBGArkKGrAMRRnj",screen:"EHjrGDHNHzgNjtE"},"iphone-18-pro-max":{body:"wUOhcMgiBmCgGaw",screen:"EHjrGDHNHzgNjtE"},"ipad-pro-13-m5":{body:"zRrSLDpdYmKeRJQ",screen:"lsDiIbtoSGSmWWZ"}};function Nd(s){let e=pl(s),t=Object.keys(e),n=t.filter(o=>o.endsWith(".usdc")&&o.includes("/")),i={};for(let o of[...n,...t.filter(a=>!n.includes(a))])i[o]=e[o];let r=Id(i,{level:0});return new yl().parse(r.buffer)}function tv(s,e){let t=new Ze().set(0,0,0,0,0,0,0,0,0),n=[new L,new L,new L],i=t.elements;s.forEach((a,l)=>{for(let c=0;c<3;c++){for(let u=0;u<3;u++)i[u*3+c]+=a[c]*a[u];n[c].addScaledVector(e[l],a[c])}});let r=t.clone().invert().elements,o=a=>new L().addScaledVector(n[0],r[0+a]).addScaledVector(n[1],r[3+a]).addScaledVector(n[2],r[6+a]);return{right:o(0).normalize(),up:o(1).normalize()}}function Ud(s,e){let t=ev[e];if(!t)throw new Error(`No nodes known for model ${e}`);s.updateMatrixWorld(!0);let n=s.getObjectByName(t.body);if(!n)throw new Error(`Model ${e} has no body ${t.body}`);let i=[];n.traverse(M=>{M instanceof ke&&i.push(M)});let r=i.find(M=>M.name===t.screen);if(!r)throw new Error(`Model ${e} has no screen ${t.screen}`);let o=r.geometry.getAttribute("position"),a=r.geometry.getAttribute("uv"),l=[],c=[];for(let M=0;M<o.count;M++)l.push(new L().fromBufferAttribute(o,M).applyMatrix4(r.matrixWorld)),c.push([a.getX(M),a.getY(M),1]);let{right:u,up:h}=tv(c,l),d=new L().crossVectors(u,h).normalize(),f=new ze().makeBasis(u,h,d).transpose(),m=l.map(M=>M.clone().applyMatrix4(f)),_=m.reduce((M,C)=>M.min(C),m[0].clone()),p=m.reduce((M,C)=>M.max(C),m[0].clone()),g=_.clone().add(p).multiplyScalar(.5),S=2.2/(p.y-_.y),v=new ze().makeTranslation(0,0,.043).multiply(new ze().makeScale(S,S,S)).multiply(new ze().makeTranslation(-g.x,-g.y,-g.z)).multiply(f),x=new ut;for(let M of i)M.geometry=M.geometry.clone().applyMatrix4(new ze().multiplyMatrices(v,M.matrixWorld)),M.position.set(0,0,0),M.quaternion.identity(),M.scale.set(1,1,1),x.add(M);let b=r.geometry.getAttribute("position"),w=0;for(let M=0;M<b.count;M++)w+=b.getZ(M);let y=w/b.count;for(let M=0;M<b.count;M++)b.setZ(M,y);b.needsUpdate=!0,r.name="device-screen";for(let M of i)M.geometry=mh(M.geometry,Math.PI/6);return x}var vl={left:"upTUAKvMVkPOMKq",right:"SiftyleUEEZwLhF",cover:"hhgAIoCGsHXeDPY",across:{JnJdTkxbQgUtLwU:"inner-bezel",UXtsBZYlaUvHoEh:"inner-display",MvKPXGSdYDVvSpk:"hinge",xdyyaajWsatVNxN:"hinge-cap"}};function nv(s,e){let t=s.geometry.index?s.geometry.toNonIndexed():s.geometry,n=t.getAttribute("position"),i=Object.keys(t.attributes),r=[0,1].map(()=>Object.fromEntries(i.map(o=>[o,[]])));for(let o=0;o<n.count;o+=3){let a=n.getX(o)+n.getX(o+1)+n.getX(o+2)<0?0:1;for(let l of i){let c=t.getAttribute(l);for(let u=o;u<o+3;u++)for(let h=0;h<c.itemSize;h++)r[a][l].push(c.array[u*c.itemSize+h])}}return r.map((o,a)=>{let l=new Lt;for(let u of i)l.setAttribute(u,new ct(o[u],t.getAttribute(u).itemSize));let c=new ke(l,s.material);return c.name=`${e}-${a===0?"left":"right"}`,c})}function Fd(s){s.updateMatrixWorld(!0);let e=s.getObjectByName(vl.left),t=s.getObjectByName(vl.right);if(!e||!t)throw new Error("iPhone Duo model has no halves");let n=new ze().makeRotationX(Math.PI/2),i=h=>(h.geometry=h.geometry.clone().applyMatrix4(new ze().multiplyMatrices(n,h.matrixWorld)),h.position.set(0,0,0),h.quaternion.identity(),h.scale.set(1,1,1),h),r=[new ut,new ut];r[0].name="left-half",r[1].name="right-half";let o=h=>{let d=[];return h.traverse(f=>{f instanceof ke&&d.push(f)}),d};for(let[h,d]of[e,t].entries())for(let f of o(d))r[h].add(i(f));for(let h of o(s)){let d=vl.across[h.name];d&&nv(i(h),d).forEach((f,m)=>r[m].add(f))}let a=r[0].getObjectByName(vl.cover),l=r[0].getObjectByName("inner-display-left");if(!a||!(l instanceof ke))throw new Error("iPhone Duo model has no cover or inner display");a.name="cover-display",l.geometry.computeBoundingBox();let c=-l.geometry.boundingBox.max.z;for(let h of r)for(let d of o(h))d.geometry.translate(0,0,c);for(let h of r)for(let d of o(h))d.geometry=mh(d.geometry,Math.PI/6);let u=new ut;return u.name="iphone-duo",u.add(...r),u}var ro={id:"ios-phone",bezel:.055,bodyRadius:.15,screenRadius:.105,depth:.085,backColor:4344669,buttons:[{edge:"right",offset:.35,length:.3},{edge:"left",offset:.48,length:.18},{edge:"left",offset:.22,length:.18}],camera:{width:.39,height:.44,insetX:.25,insetY:.29,lensRadius:.068,lenses:[[-.08,.095],[.08,-.095]],flash:[.085,.11]}},Od={id:"ios-tablet",bezel:.065,bodyRadius:.105,screenRadius:.045,depth:.055,backColor:10266031,buttons:[{edge:"top",offset:.5,length:.15},{edge:"right",offset:.78,length:.13},{edge:"right",offset:.58,length:.13}],camera:{width:.19,height:.19,insetX:.15,insetY:.15,lensRadius:.045,lenses:[[0,0]],flash:null}},iv={id:"android-phone",bezel:.035,bodyRadius:.115,screenRadius:.08,depth:.085,backColor:3425353,buttons:[{edge:"right",offset:.2,length:.24},{edge:"right",offset:.65,length:.32}],camera:{width:.24,height:.47,insetX:.18,insetY:.29,lensRadius:.056,lenses:[[0,.11],[0,-.11]],flash:[.1,0]}},sv={...Od,id:"android-tablet",backColor:6912896};function Bd(s){let e=s.name??"",t=/\b(ipad|tablet)\b/i.test(e),n=/\b(iphone|phone)\b/i.test(e),i=t||!n&&s.portraitAspect>=.6;return s.platform==="ios"?i?Od:ro:i?sv:iv}var js=2.2;function oo(s,e,t,n=new kn){let i=-s/2,r=-e/2;return n.moveTo(i+t,r),n.lineTo(i+s-t,r),n.quadraticCurveTo(i+s,r,i+s,r+t),n.lineTo(i+s,r+e-t),n.quadraticCurveTo(i+s,r+e,i+s-t,r+e),n.lineTo(i+t,r+e),n.quadraticCurveTo(i,r+e,i,r+e-t),n.lineTo(i,r+t),n.quadraticCurveTo(i,r,i+t,r),n}function Ih(s,e,t){let n=e||s?.width||900,i=t||s?.height||1950,r=s?.orientation==="landscape_left"||s?.orientation==="landscape_right",o=s?.orientation==="landscape_left"?-Math.PI/2:s?.orientation==="landscape_right"?Math.PI/2:s?.orientation==="portrait_upside_down"?Math.PI:0;return{aspect:Math.min(n,i)/Math.max(n,i),rotation:o,landscape:r,rawLandscape:n>i}}function lo(s,e,t=ro){let{aspect:n}=e,i=new ut,r=new ut;i.add(r);let o=js*n,a=o+t.bezel*2,l=js+t.bezel*2,c=.01-t.depth,u=new fn({color:11910343,metalness:.88,roughness:.27}),h=new nn({color:1316896,metalness:.15,roughness:.2,clearcoat:1}),d=new fn({color:t.backColor,metalness:.45,roughness:.32}),f=new nn({color:463654,metalness:.6,roughness:.12,clearcoat:1}),m=[u,h,d,f],_=new ke(new li(oo(a,l,t.bodyRadius),{depth:t.depth,bevelEnabled:!0,bevelSize:.012,bevelThickness:.012,bevelSegments:3,steps:1,curveSegments:12}),u);_.position.z=.025-t.depth,r.add(_);let p=new ke(new yn(oo(a-.014,l-.014,t.bodyRadius-.01),16),h);p.position.z=.04,r.add(p);let g=new ke(new yn(oo(a-.012,l-.012,t.bodyRadius-.01),16),d);g.name="device-back",g.rotation.y=Math.PI,g.position.z=c,r.add(g);let S=new yn(oo(o,js,t.screenRadius),20);ao(S,o,js,e);let v=new Wt({map:s,toneMapped:!1}),x=new ke(S,v);x.position.z=.043,r.add(x);for(let{edge:D,offset:k,length:P}of t.buttons){let O=new ke(new sn(D==="top"?P:.026,D==="top"?.026:P,t.depth*.65),u);O.position.set(D==="top"?k:(D==="left"?-1:1)*(a/2+.015),D==="top"?l/2+.015:k,(.025+c)/2),r.add(O)}let b=new ut;b.name="rear-camera",b.position.set(a/2-t.camera.insetX,l/2-t.camera.insetY,g.position.z+.001),b.rotation.y=Math.PI,r.add(b);let w=.025+.007,y=.025,M=new ke(new li(oo(t.camera.width,t.camera.height,Math.min(t.camera.width,t.camera.height)/4),{depth:.025,bevelEnabled:!0,bevelSize:.009,bevelThickness:.007,bevelSegments:2}),d);M.name="camera-plate",b.add(M);for(let[D,k]of t.camera.lenses){let P=t.camera.lensRadius,O=new ke(new Rn(P+.014,P+.014,.025,32),u);O.rotation.x=Math.PI/2,O.name="camera-ring",O.position.set(D,k,w+y/2-.003),b.add(O);let G=new ke(new ai(P,32),f);G.name="camera-lens",G.position.set(D,k,O.position.z+y/2+5e-4),b.add(G)}let C=new Wt({color:15919830});if(t.camera.flash){let D=new ke(new ai(.022,20),C);D.position.set(t.camera.flash[0],t.camera.flash[1],w+5e-4),b.add(D)}let E=e,R=Dh(x,r,o,js,()=>E);return{root:i,orientation:r,width:a,height:l,screenPoint:R,setDisplay(D,k){E=k,v.map=D,ao(S,o,js,E)},dispose(){i.traverse(D=>{D instanceof ke&&D.geometry.dispose()});for(let D of m)D.dispose();v.dispose(),C.dispose()}}}function ao(s,e,t,n){let i=s.getAttribute("position");s.hasAttribute("uv")||s.setAttribute("uv",new ct(new Float32Array(i.count*2),2));let r=s.getAttribute("uv");for(let o=0;o<i.count;o++){let a=(i.getX(o)+e/2)/e,l=(i.getY(o)+t/2)/t;r.setXY(o,n.rawLandscape?n.rotation>0?1-l:l:a,n.rawLandscape?n.rotation>0?a:1-a:l)}r.needsUpdate=!0}function Dh(s,e,t,n,i){let r=new ui,o=new pe,a=new L;s.geometry.computeBoundingBox();let l=s.position.z+(s.geometry.boundingBox?.max.z??0),c=new tn(new L(0,0,1),-l);return(u,h,d,f=!1)=>{if(e.updateWorldMatrix(!0,!0),d.updateMatrixWorld(!0),o.set(u*2-1,1-h*2),r.setFromCamera(o,d),f){if(!r.ray.clone().applyMatrix4(e.matrixWorld.clone().invert()).intersectPlane(c,a))return null}else{let g=r.intersectObject(s,!1)[0];if(!g)return null;a.copy(g.point),e.worldToLocal(a)}let m=Math.min(1,Math.max(0,(a.x+t/2)/t)),_=Math.min(1,Math.max(0,(a.y+n/2)/n)),{rotation:p}=i();return p===-Math.PI/2?{x:_,y:m}:p===Math.PI/2?{x:1-_,y:1-m}:p===Math.PI?{x:1-m,y:_}:{x:m,y:1-_}}}function rv(s){let e=new Set,t=new Set,n=new Set;s.traverse(r=>{if(r instanceof ke){e.add(r.geometry);for(let o of Array.isArray(r.material)?r.material:[r.material]){t.add(o);for(let a of Object.values(o))a instanceof Nt&&n.add(a)}}});for(let r of e)r.dispose();for(let r of t)r.dispose();let i=new Set;for(let r of n)r.dispose(),typeof ImageBitmap<"u"&&r.image instanceof ImageBitmap&&i.add(r.image);for(let r of i)r.close()}function zd(s,e,t){let n=[];s.traverse(_=>{_ instanceof ke&&_.name==="device-screen"&&n.push(_)});let i=n[0];if(!i||n.length!==1)throw new Error("Device model must have one device-screen mesh");s.updateMatrixWorld(!0);let r=new Mt().setFromObject(i),o=r.max.x-r.min.x,a=r.max.y-r.min.y;if(i.matrixWorld.elements.some((_,p)=>!Number.isFinite(_)||Math.abs(_-(p%5===0?1:0))>.001)||!Number.isFinite(o)||!Number.isFinite(a)||!Number.isFinite(r.min.z)||!Number.isFinite(r.max.z)||o<=0||Math.abs(a-2.2)>.001||Math.abs(r.min.x+r.max.x)>.001||Math.abs(r.min.y+r.max.y)>.001||r.max.z-r.min.z>.001)throw new Error("Device model display is not normalized");ao(i.geometry,o,a,t);let l=i.material,c=new Wt({map:e,toneMapped:!1});i.material=c;let u=new ut,h=new ut;h.add(s),u.add(h);let d=new Mt().setFromObject(s),f=t,m=Dh(i,h,o,a,()=>f);return{root:u,orientation:h,width:d.max.x-d.min.x,height:d.max.y-d.min.y,setDisplay(_,p){f=p,c.map=_,ao(i.geometry,o,a,f)},screenPoint:m,dispose(){i.material=l,c.map=null,c.dispose(),h.remove(s)}}}async function co(s,e){let t=await fetch(s.url,{signal:e});if(!t.ok)throw new Error(`Device model request failed: ${t.status}`);let n=await t.arrayBuffer();e.throwIfAborted();let i=Nd(new Uint8Array(n)),r=s.id==="iphone-duo"?Fd(i):Ud(i,s.id);return{asset:r,dispose:()=>rv(r)}}function kd(s,e){return s!=="ios"?null:/^iPhone Duo$/i.test(e)?"iphone-duo":/^iPhone 18 Pro Max$/i.test(e)?"iphone-18-pro-max":/^iPhone 18 Pro$/i.test(e)?"iphone-18-pro":/^iPad Pro 13-inch \(M5\)$/i.test(e)?"ipad-pro-13-m5":null}function ho(s){let e=null,t=null,n=!1,i=()=>{t&&s.install(null),t?.dispose(),t=null};return{set(r){if(n||r?.id===e?.source.id&&r?.url===e?.source.url||(e?.controller.abort(),e=null,i(),!r))return;let o={source:r,controller:new AbortController};e=o,s.load(r,o.controller.signal).then(a=>{if(n||e!==o){a.dispose();return}try{s.install(a),t=a}catch(l){s.install(null),a.dispose(),e=null,s.onError?.(l)}},a=>{!n&&e===o&&(e=null,s.onError?.(a))})},dispose(){n||(n=!0,e?.controller.abort(),e=null,i())}}}function Sl(){let s=new L,e=new L,t=new L,n=new Mt,i=1,r=1,o=0,a=1,l=1,c=0,u=!1,h=!1,d=!1,f=(m,_=!1)=>{if(d)return!1;let p=Math.max(0,(m-c)/1e3);if(c=m,!h)return!1;let g=14,S=Math.exp(-g*p),v=s.clone().sub(e),x=t.clone().addScaledVector(v,g);s.copy(v).addScaledVector(x,p).multiplyScalar(S).add(e),t.addScaledVector(x,-g*p).multiplyScalar(S);let b=i-r,w=o+g*b;return i=r+(b+w*p)*S,o=(o-g*w*p)*S,h=s.distanceTo(e)>5e-4||t.length()>.005||Math.abs(i-r)>5e-4||Math.abs(o)>.005,(_||!h)&&(s.copy(e),i=r,t.set(0,0,0),o=0,h=!1),!0};return{center:s,setBounds(m,_,p,g,S=!1){if(m.isEmpty())return;f(g),n.copy(m),l=Math.tan(_),a=l*p,n.getCenter(e),e.z=0;let v=n.getSize(new L);r=Math.max(1,Math.max(v.x/a,v.y/l)*.565+n.max.z),h=!0,f(g,S||!u),u=!0},advance:f,distance(){let m=Math.max(Math.abs(n.min.x-s.x)/a,Math.abs(n.max.x-s.x)/a,Math.abs(n.min.y-s.y)/l,Math.abs(n.max.y-s.y)/l);return Math.max(i,m*1.02+n.max.z,1)},hold(m,_){d=m,c=_},needsFrame(){return h&&!d}}}function Ml(s){let e=s.clone().normalize();e.w<0&&e.set(-e.x,-e.y,-e.z,-e.w);let t=Math.hypot(e.x,e.y,e.z);return t<1e-8?new L:new L(e.x,e.y,e.z).multiplyScalar(2*Math.atan2(t,e.w)/t)}function is(s){let e=s.length();return e<1e-8?new st:new st().setFromAxisAngle(s.clone().divideScalar(e),e)}var Vd=.006,ov=.5,av=.78,Uh=2*Math.PI/ov,lv=Uh*av,uo=1/120,cv=4.5,Gd=22,Lh=1.1,Nh=2;function bl(s){let e=new st,t=new st,n=new L,i=new L,r=null,o=null,a=null,l=!1,c=!1,u=-1/0,h=(_,p)=>{t.copy(_).normalize(),r={at:p,rotation:e.clone(),velocity:n.clone(),steps:0}},d=_=>{if(!a)return;_-u>100&&i.set(0,0,0);let p=t.clone().premultiply(is(i.clone().multiplyScalar(.085)));n.multiplyScalar(.2).addScaledVector(i,.8).clampLength(0,Gd);let g=a.x!==0||a.y!==0,S=a.rest;if(a=null,g&&n.length()>=cv){let v=e.clone().premultiply(is(n.clone().multiplyScalar(1/Lh)));t.copy(s.choose(v)).normalize(),o={at:_,rotation:e.clone(),velocity:n.clone(),correction:Ml(t.clone().multiply(v.invert()))};return}h(g?s.choose(p):S,_)},f=(_,p=!1)=>{if(l||!Number.isFinite(_))return!1;if(o){let y=Math.max(0,(_-o.at)/1e3),M=Math.exp(-Lh*y),C=Math.exp(-Nh*y),E=is(o.correction.clone().multiplyScalar(1-(1+Nh*y)*C));return e.copy(o.rotation).premultiply(is(o.velocity.clone().multiplyScalar((1-M)/Lh))).premultiply(E).normalize(),n.copy(o.velocity).multiplyScalar(M).applyQuaternion(E).addScaledVector(o.correction,Nh**2*y*C),(p||e.angleTo(t)<.01&&n.length()<.08)&&(o=null,e.copy(t),n.set(0,0,0)),!0}if(a){let y=Math.min(.05,Math.max(0,(_-a.at)/1e3));a.at=_;let M=Math.ceil(y*120),C=M?y/M:0;for(let E=0;E<M;E++){let R=Ml(t.clone().multiply(e.clone().invert()));if(R.lengthSq()>1e-10){let P=R.clone().normalize(),O=Math.round((a.error.dot(P)-R.length())/(2*Math.PI));R.addScaledVector(P,O*2*Math.PI)}a.error.copy(R);let D=2*Math.PI/.68,k=R.clampLength(0,1.2).multiplyScalar(D*D).addScaledVector(n,-2*1.12*D);n.addScaledVector(k,C).clampLength(0,9),e.premultiply(is(n.clone().multiplyScalar(C))).normalize()}return p&&(e.copy(t),n.set(0,0,0)),M>0||p}if(!r)return!1;let g=Math.max(0,(_-r.at)/1e3),S=Math.min(240,Math.floor(g/uo)),v=(y,M,C)=>{let R=Ml(t.clone().multiply(y.clone().invert())).multiplyScalar(Uh*Uh).addScaledVector(M,-2*lv);M.addScaledVector(R,C).clampLength(0,9),y.premultiply(is(M.clone().multiplyScalar(C))).normalize()};for(;r.steps<S;r.steps++)v(r.rotation,r.velocity,uo);e.copy(r.rotation),n.copy(r.velocity);let x=r.rotation.clone(),b=r.velocity.clone();v(x,b,uo);let w=Math.min(1,(g-S*uo)/uo);return e.slerp(x,w),n.lerp(b,w),(p||S===240||e.angleTo(t)<5e-4&&n.length()<.005)&&(e.copy(t),n.set(0,0,0),r=null),!0},m=_=>{l||a||(f(_),a={start:e.clone(),rest:t.clone(),x:0,y:0,error:new L,at:_},t.copy(e),i.set(0,0,0),u=-1/0,r=null,o=null)};return{rotation:e,setPose(_,p,g=!1){f(p),a=null,o=null,h(_,p),g&&(e.copy(t),n.set(0,0,0),r=null)},dragActive(_,p){_?((a||o)&&(f(p),a=null,o=null),m(p)):(f(p),d(p))},orbit(_,p,g){if(l||![_,p,g].every(Number.isFinite)||!_&&!p||(f(g),m(g),!a))return;let S=t.clone();a.x+=_,a.y+=p,t.copy(is(new L(a.y*Vd,a.x*Vd,0))).multiply(a.start).normalize();let v=(g-u)/1e3;i.copy(Ml(t.clone().multiply(S.invert()))),v>0&&v<.1?i.divideScalar(v).clampLength(0,Gd):i.set(0,0,0),u=g},hold(_,p){l!==_&&(_&&(c=a!==null||o!==null,a=null,o=null,r=null,n.set(0,0,0)),l=_,_||(c?h(s.choose(e.clone()),p):e.angleTo(t)>5e-4&&h(t,p),c=!1))},advance:f,needsFrame(){return!l&&(r!==null||o!==null||a!==null&&(e.angleTo(t)>5e-4||n.length()>.005))},reset(_,p){f(p),a=null,o=null,h(_,p)}}}var hv=new L(0,1,0);function wl(s,e){let t=null,n=1/0;for(let i of e){let r=s.clone().multiply(i.rotation.clone().invert()),o=2*Math.atan2(r.y,r.w),a=Math.max(-i.yawLimit,Math.min(i.yawLimit,Math.atan2(Math.sin(o),Math.cos(o)))),l=i.rotation.clone().premultiply(new st().setFromAxisAngle(hv,a)),c=l.angleTo(s);c<n-1e-8&&(n=c,t={...i,rotation:l})}return t}var Hd=["portrait","landscape_left","portrait_upside_down","landscape_right"],uv=new L(0,0,1),fv=new L(0,1,0);function Al(s,e){let t=[];for(let n of s){let i=n.normal.clone().normalize();if(i.lengthSq()<.5)continue;let r=n.up.clone().cross(i).normalize(),o=i.clone().cross(r).normalize(),a=new st().setFromRotationMatrix(new ze().makeBasis(r,o,i)).invert();for(let l=0;l<Hd.length;l++){if(n.face==="right"&&l!==0||n.face==="left"&&l!==2)continue;let c=(e===3?Math.PI/2:0)-l*Math.PI/2,u=a.clone().premultiply(new st().setFromAxisAngle(uv,c));if((n.face==="left"||n.face==="right")&&u.premultiply(new st().setFromAxisAngle(new L(1,0,0),Math.PI/12)),e===3&&s.some(d=>(d.face==="left"||d.face==="right")&&d.normal.clone().applyQuaternion(u).z<.04))continue;let h=n.face==="cover"?Math.PI/9:Math.PI/3;if(e===3)for(let d of[-1,1])for(let f=1;f<=60;f++){let m=new st().setFromAxisAngle(fv,d*f*Math.PI/180);if(s.some(_=>(_.face==="left"||_.face==="right")&&_.normal.clone().applyQuaternion(u).applyQuaternion(m).z<.04)){h=Math.min(h,(f-1)*Math.PI/180);break}}t.push({rotation:u,face:n.face,orientation:Hd[l],center:n.center.clone(),yawLimit:h})}}return t}var ss=wl;function Tl(s,e=requestAnimationFrame,t=cancelAnimationFrame){let n=null,i=!1;return{invalidate(){i||n!==null||(n=e(()=>{n=null,i||s()}))},dispose(){i=!0,n!==null&&t(n),n=null}}}function Wd(s){let e=[1,3].map(V=>{let N=document.createElement("canvas");N.width=V===1?784:1600,N.height=V===1?1140:1125;let X=N.getContext("2d",{alpha:!1});if(!X)throw new Error("Duo display canvas is unavailable");return X.fillStyle="#080a10",X.fillRect(0,0,N.width,N.height),{id:V,canvas:N,context:X}}).map(({id:V,canvas:N,context:X})=>{let j=new Hi(N);return j.colorSpace=wt,j.minFilter=j.magFilter=Dt,j.generateMipmaps=!1,{id:V,canvas:N,context:X,texture:j}}),t=new Zs({canvas:s.canvas,alpha:!0,antialias:!0,powerPreference:"low-power"});t.outputColorSpace=wt;let n=new si,i=(()=>{let V=new Ys(t),N=new ul;try{return V.fromScene(N,.04)}catch(X){for(let j of e)j.texture.dispose();throw t.dispose(),t.forceContextLoss(),X}finally{N.dispose(),V.dispose()}})();n.environment=i.texture;let r=new Ct(36,1,.1,150),o=new Sn(16777215,4);o.position.set(-6,10,14);let a=new Sn(13098239,2);a.position.set(8,-3,-5),n.add(new Zi(16777215,2.4),o,a);let l=null,c=null,u="",h="",d=0,f=!1,m={width:0,height:0,ratio:1},_={width:0,height:0,ratio:0},p="inside",g=null,S=null,v=new L,x=new L,b=null,w=null,y=()=>{w&&clearTimeout(w),w=null,b=null},M=()=>l?Al(l.restFrames(c?.screenId===1?1:3),c?.screenId===1?1:3):[],C=()=>{if(!l||!c?.supportsPhysicalOrientation||!s.onPanelRequested)return M();let V=Al(l.restFrames(1),1);return R>20&&R<180?[...Al(l.restFrames(3),3),...V]:M()},E=bl({choose(V){let N=ss(V,C());if(!N)return V;p=N.face;let X=N.face==="cover"?1:3;return c&&X!==(b??c.screenId)&&s.onPanelRequested?(y(),b=X,l?.cancelInput(),s.onPanelRequested(X),w=setTimeout(()=>{y();let j=ss(E.rotation,M());j&&(p=j.face,E.setPose(j.rotation,performance.now())),se.invalidate()},5e3)):c&&b===null&&N.orientation!==S&&s.onOrientationRequested&&(S=N.orientation,g=N.orientation,s.onOrientationRequested(N.orientation)),N.rotation}}),R=180,D=180,k=null,P=null,O=Number.NaN,G=!1,U=Sl(),ee=new Mt,W=!0,K=!1,$="open",he=180,de=new st,q=0,ie=typeof matchMedia=="function"&&matchMedia("(prefers-reduced-motion: reduce)").matches,ce=()=>{if(!l)return;let V=P&&O!==R?l.leafRotation(P):null;if(l.setAngle(R),O=R,V&&P){let j=V.multiply(l.leafRotation(P).invert());j.angleTo(new st)>1e-8&&E.setPose(E.rotation.clone().multiply(j),performance.now(),!0)}l.root.quaternion.copy(E.rotation),l.root.position.set(0,0,0);let N=l.restFrames(p==="cover"?1:3),X=N.find(j=>j.face===p)??N[0];x.copy(X?.center??new L),l.root.position.copy(v).applyQuaternion(l.root.quaternion).negate()},Ce=()=>{r.position.set(U.center.x,U.center.y,U.distance()),r.lookAt(U.center.x,U.center.y,0),r.updateProjectionMatrix()},ue=(V=!1)=>{!l||!m.width||!m.height||(r.aspect=m.width/m.height,U.setBounds(ee.setFromObject(l.root),r.fov*Math.PI/360,r.aspect,performance.now(),V),Ce())},H=()=>Math.abs(R-D)>.01||v.distanceTo(x)>.001,se=Tl(()=>{if(!(f||!l||!m.width||!m.height))try{let V=performance.now(),N=Math.max(0,(V-q)/1e3),X=ie?1:1-Math.exp(-14*Math.max(0,(V-q)/1e3||.016));q=V;let j=H(),ge=E.advance(V,ie);j&&(R+=(D-R)*X,Math.abs(R-D)<=.01&&(R=D),ce(),v.lerp(x,X),v.distanceTo(x)<=.001&&v.copy(x),ce(),ue(ie||N>.5)),ge&&(ce(),ue(ie||N>.5)),U.advance(V,ie)&&Ce(),(_.width!==m.width||_.height!==m.height||_.ratio!==m.ratio)&&(t.setDrawingBufferSize(m.width,m.height,m.ratio),_=m),t.render(n,r),(j||H()||E.needsFrame()||U.needsFrame())&&se.invalidate()}catch{s.onUnavailable()}}),fe=ho({load:co,onError(V){s.onModelError?.(V),s.onUnavailable()},install(V){let N=V?gd(V.asset,{1:e[0].texture,3:e[1].texture}):null;if(l&&(n.remove(l.root),l.dispose()),l=N,O=Number.NaN,!(!l||f)){if(n.add(l.root),ce(),c?.screenId===1&&$!=="tent"){let X=ss(E.rotation,M());X&&(p=X.face,E.setPose(X.rotation,performance.now(),!0),ce())}v.copy(x),ce(),ue(),se.invalidate(),s.onModelReady?.()}}}),Le=V=>{V.preventDefault(),s.onUnavailable()};return s.canvas.addEventListener("webglcontextlost",Le),fe.set(s.model),{setScreen(V){if(f)return;let N=H()||E.needsFrame();Ks(c)!==Ks(V)&&(u="",d=performance.now(),l?.cancelInput());let X=c,j=b!==null;(!V||V.screenId===b)&&y();let ge=g!==null,le=ge&&V?.screenId===X?.screenId,Re=V?.screenId!==X?.screenId;g=null,c=V;let xe=V?.hingePose&&V.hingePose!==X?.hingePose,Ue=P!==null&&!xe,Je=X?.hingeAngle??(X?.screenId===1?0:180),Qe=V?.hingeAngle??(V?.screenId===1?0:180);!V||Ue||xe?K=!1:X&&Je!==Qe&&(K=!0);let B=K&&!!V&&yo(V)&&!j;V&&yo(V)&&(K=!1);let xt=!K&&V?.screenId===X?.screenId&&V?.hingeAngle===X?.hingeAngle&&V?.orientation!==X?.orientation,je=xt&&!le&&!j&&!Ue&&!V?.hingePose&&k===null;if((W||xe||je||B)&&($=V?.hingePose??(V?.screenId===1?"closed":"open"),he=V?.hingeAngle??(V?.screenId===1?0:180)),D=k??V?.hingeAngle??(V?.screenId===1?0:180),W||xe||B||xt&&!le&&!j&&!Ue){P=null,S=V?.orientation??null;let rt=he,Me=(180-rt)*Math.PI/360,yt=$==="closed"?0:Math.PI/2;V&&V.width<V.height&&(V.orientation==="landscape_left"&&(yt-=Math.PI/2),V.orientation==="landscape_right"&&(yt+=Math.PI/2)),V?.orientation==="portrait_upside_down"&&(yt-=Math.PI);let He=$==="laptop"?new Ut(-Me+Math.PI/9,-Math.PI/9,Math.PI/2,"YXZ"):$==="tent"?new Ut(Math.PI/2+Math.PI/18,-Math.PI/9,-Math.PI/2,"YXZ"):new Ut(0,Math.PI/2*Math.pow(1-rt/180,3),0,"YXZ");if(de=new st().setFromEuler(He),$!=="laptop"&&$!=="tent"&&de.premultiply(new st().setFromAxisAngle(new L(0,0,1),yt)),V?.screenId===1&&!B&&$!=="tent"){let qe=ss(de,M());qe&&de.copy(qe.rotation)}W&&(R=D),p=V?.screenId===1?"cover":$==="laptop"?"right":"inside",E.setPose(de,performance.now(),W),W=!1}else if(V&&Re&&!j&&!Ue&&p==="cover"!=(V.screenId===1)&&(ge||yo(V))){let rt=ss(E.rotation,M());rt&&(p=rt.face,E.setPose(rt.rotation,performance.now()))}N||(q=performance.now()),ce(),ue(),se.invalidate()},setHingePreview(V,N=!1){f||V!==null&&(!Number.isFinite(V)||V<0||V>180)||(H()||(q=performance.now()),k=V,V!==null&&!N?P=null:V!==null&&!P&&l&&(y(),g=null,P=c?.screenId===1&&R>20?"left":"right",E.setPose(E.rotation.clone(),performance.now(),!0)),E.hold(G||N&&V!==null,performance.now()),D=V??c?.hingeAngle??(c?.screenId===1?0:180),V!==null&&N&&(R=V,ce(),ue()),l?.cancelInput(),se.invalidate())},setInteractionActive(V,N="touch"){f||(N==="orbit"?(V&&(P=null),E.dragActive(V,performance.now())):(G=V,E.hold(V||k!==null,performance.now()),U.hold(V,performance.now())),se.invalidate())},rejectOrientation(){if(f||g===null&&b===null)return;y(),g=null,S=c?.orientation??null;let V=M().filter(X=>X.orientation===c?.orientation),N=ss(E.rotation,V);N&&(p=N.face,E.setPose(N.rotation,performance.now())),se.invalidate()},frameUpdated(V,N){if(f||c?.screenId!==V)return;let X=Ks(c);if(!N&&h===X)return;let j=N??s.sources[V];if(!md(j,c))return;let ge=e[V===1?0:1],{context:le,canvas:Re}=ge;if(!N&&!u&&performance.now()-d<1500){let Je=document.createElement("canvas");Je.width=Je.height=8;let Qe=Je.getContext("2d",{willReadFrequently:!0});if(Qe&&(Qe.drawImage(j,0,0,8,8),!Qe.getImageData(0,0,8,8).data.some((B,xt)=>xt%4!==3&&B>3)))return}le.save(),le.fillStyle="#080a10",le.fillRect(0,0,Re.width,Re.height),le.translate(Re.width/2,Re.height/2);let xe=V===3;xe&&le.rotate(Math.PI/2);let Ue=Math.min(Re.width/(xe?j.height:j.width),Re.height/(xe?j.width:j.height));le.drawImage(j,-j.width*Ue/2,-j.height*Ue/2,j.width*Ue,j.height*Ue),le.restore(),ge.texture.needsUpdate=!0,u=Ks(c),N&&(h=u),se.invalidate()},resize(V,N,X){f||![V,N,X].every(Number.isFinite)||V<=0||N<=0||(m={width:V,height:N,ratio:Math.min(2,Math.max(1,X))},ue(!0),se.invalidate())},screenPoint(V,N,X=!1){return f||H()||k!==null||g!==null||b!==null?null:(ce(),l?.screenPoint(V,N,r,c,u,X)??null)},cancelInput(){P=null,l?.cancelInput()},orbit(V,N){f||(P=null,E.orbit(V*m.width,N*m.height,performance.now()),se.invalidate())},beginHinge(V,N){return f||!l||!c||(ce(),!l.hingeLeafAt(V,N,r))?!1:(y(),g=null,P=c.screenId===1&&R>20?"left":"right",E.setPose(E.rotation.clone(),performance.now(),!0),l.cancelInput(),!0)},resetPose(){if(f)return;P=null,g=null,y();let V=M().find(N=>N.orientation===c?.orientation);V&&V.orientation!==S&&s.onOrientationRequested&&(S=V.orientation,g=V.orientation,s.onOrientationRequested(V.orientation)),E.reset(V?.rotation??de,performance.now()),ce(),ue(),se.invalidate()},dispose(){if(y(),!f){f=!0,se.dispose(),fe.dispose(),s.canvas.removeEventListener("webglcontextlost",Le);for(let V of e)V.texture.dispose();n.environment=null,i.dispose(),t.dispose(),t.forceContextLoss()}}}}function Xd(s){let{deltaX:e,deltaY:t,deltaMode:n,ctrlKey:i,width:r,height:o}=s;if(![e,t,r,o].every(Number.isFinite)||r<=0||o<=0||n!==0&&n!==1&&n!==2)return null;let a=n===1?16:n===2?o:1,l=(c,u)=>Math.min(u,Math.max(-u,c));return i?{type:"zoom",delta:l(-t*a*.01,1)}:{type:"orbit",x:l(-e*a/r,.25),y:l(-t*a/o,.25)}}function qd(s){let e=null;return{navigate(t){return e?!1:(t.type==="zoom"?s.zoomBy(t.delta):s.orbit(t.x,t.y),!0)},endWheel(){e||s.onInteractionActive?.(!1,"orbit")},begin(t,n,i=!1){if(e)return!1;let r=i?null:s.screenPoint(n,!1);return e={id:t,mode:r?"touch":"orbit",last:n,screen:r},s.onInteractionActive?.(!0,e.mode),r&&s.touch("begin",r),!0},move(t,n){if(e?.id===t){if(e.mode==="touch"){let i=s.screenPoint(n,!0);i&&(e.screen=i,s.touch("move",i))}else s.orbit(n.x-e.last.x,n.y-e.last.y);e.last=n}},end(t){if(!e||t!==void 0&&e.id!==t)return;let n=e;e=null,n.mode==="touch"&&n.screen&&s.touch("end",n.screen),s.onInteractionActive?.(!1,n.mode)}}}function Yd(s,e,t){let n=null,i=!1,r=null,o=!1,a=null,l=()=>{a&&clearTimeout(a),a=null,o&&(o=!1,e.endWheel())},c=()=>{r&&clearTimeout(r),r=null,i=!1,n=null,t?.end()},u=g=>{let S=s.getBoundingClientRect(),v="clientX"in g&&typeof g.clientX=="number"?g.clientX:NaN,x="clientY"in g&&typeof g.clientY=="number"?g.clientY:NaN;t?.begin((v-S.left)/S.width,(x-S.top)/S.height)},h=g=>{g.preventDefault(),g.stopPropagation()},d=g=>{if(h(g),n!==null)return;if(g.ctrlKey){l(),i||(u(g),i=!0);let x=g.deltaMode===1?16:g.deltaMode===2?s.getBoundingClientRect().height:1;t?.move(-g.deltaY*x/100),r&&clearTimeout(r),r=setTimeout(c,180);return}if(i&&c(),"momentum"in g&&g.momentum===!0){l();return}let S=s.getBoundingClientRect(),v=Xd({width:S.width,height:S.height,deltaX:g.deltaX,deltaY:g.deltaY,deltaMode:g.deltaMode,ctrlKey:g.ctrlKey});v&&e.navigate(v)&&(o=!0,a&&clearTimeout(a),a=setTimeout(l,1200))},f=g=>!("scale"in g)||typeof g.scale!="number"||!Number.isFinite(g.scale)||g.scale<=0?null:g.scale,m=g=>{h(g),l(),c(),u(g),n=f(g)??1},_=g=>{h(g);let S=f(g);n===null||S===null||(t?.move(Math.log(S/n)),n=S)},p=g=>{h(g),c()};return s.addEventListener("wheel",d,{passive:!1}),s.addEventListener("gesturestart",m,{passive:!1}),s.addEventListener("gesturechange",_,{passive:!1}),s.addEventListener("gestureend",p,{passive:!1}),{endOrbit:l,cancel(){l(),c()},dispose(){s.removeEventListener("wheel",d),s.removeEventListener("gesturestart",m),s.removeEventListener("gesturechange",_),s.removeEventListener("gestureend",p),l(),c()}}}var os=2.2,Un=.075,er=.026,Zd=.004,Qs=.008,rs=Un/2+.005,Jd=rs+Un/2-.002,Fh=2076/2152,Oh=s=>Number.isFinite(s)&&s>.75&&s<1.5;function El(s,e,t,n,i=0){let r=e==="left"?-s+t:Zd/2+i,o=e==="left"?-Zd/2-i:s-t,a=-os/2+t,l=os/2-t,c=new kn;return e==="left"?(c.moveTo(r+n,a),c.lineTo(o,a),c.lineTo(o,l),c.lineTo(r+n,l),c.quadraticCurveTo(r,l,r,l-n),c.lineTo(r,a+n),c.quadraticCurveTo(r,a,r+n,a)):(c.moveTo(r,a),c.lineTo(o-n,a),c.quadraticCurveTo(o,a,o,a+n),c.lineTo(o,l-n),c.quadraticCurveTo(o,l,o-n,l),c.lineTo(r,l)),c.closePath(),c}function $d(s,e,t){let n=new kn;return n.moveTo(-s/2+t,-e/2),n.lineTo(s/2-t,-e/2),n.quadraticCurveTo(s/2,-e/2,s/2,-e/2+t),n.lineTo(s/2,e/2-t),n.quadraticCurveTo(s/2,e/2,s/2-t,e/2),n.lineTo(-s/2+t,e/2),n.quadraticCurveTo(-s/2,e/2,-s/2,e/2-t),n.lineTo(-s/2,-e/2+t),n.quadraticCurveTo(-s/2,-e/2,-s/2+t,-e/2),n.closePath(),n}function dv(s){return $d(s-er*2-.02,os-er*2-.04,.07)}function Kd(s,e,t,n=Fh){let i=os-2*er,r=n*i/2+er,o=new ut,a=new ut;o.add(a);let l=new ut,c=new ut;a.add(l,c);let u=new fn({color:10726322,metalness:.9,roughness:.28}),h=new fn({color:12896976,metalness:.95,roughness:.16}),d=new nn({color:724240,metalness:.1,roughness:.2,clearcoat:1}),f=new nn({color:2896439,metalness:.35,roughness:.52,clearcoat:.4,clearcoatRoughness:.6}),m=new nn({color:1711650,metalness:.55,roughness:.3,clearcoat:1}),_=new nn({color:397346,metalness:.6,roughness:.1,clearcoat:1}),p=new Wt({color:15919830}),g=new Wt({map:s,toneMapped:!1}),S=new Wt({colorWrite:!1,depthWrite:!1}),v=new Wt({map:s,toneMapped:!1}),x=[u,h,d,f,m,_,p,g,S,v];l.position.z=rs;let b=new ut;b.position.z=-rs,l.add(b);function w(N,X,j){let ge=new ke(new li(El(r,X,Qs,.1,Qs),{depth:Un-Qs*2,bevelEnabled:!0,bevelSize:Qs,bevelThickness:Qs,bevelSegments:3,curveSegments:12}),u);ge.position.z=-Un/2+Qs,N.add(ge);let le=new ke(new yn(El(r,X,.01,.095,.002),12),d);le.position.z=Un/2+.001,N.add(le);let Re=new ke(new yn(El(r,X==="left"?"right":"left",.01,.095,.002),12),j);Re.name=`${X}-back`,Re.rotation.y=Math.PI,Re.position.set(0,0,-Un/2-.001),N.add(Re);let xe=new yn(El(r,X,er,.076));xe.computeBoundingBox();let Ue=new ke(xe,S);return Ue.name=`${X}-inner-screen`,Ue.position.z=Un/2+.003,N.add(Ue),Ue}let y=w(b,"left",d),M=w(c,"right",f),C=2*(r-er),E=new Xi(C,i,40,48),R=E.getAttribute("position"),D=E.getAttribute("uv"),k=new Float32Array(R.count);for(let N=0;N<R.count;N++){let X=R.getY(N),j=C/2,ge=i/2,le=.076,Re=Math.max(0,Math.abs(X)-(ge-le)),xe=j-le+Math.sqrt(Math.max(0,le*le-Re*Re)),Ue=Math.max(-xe,Math.min(xe,R.getX(N)));k[N]=Ue,D.setXY(N,Ue/C+.5,X/i+.5)}D.needsUpdate=!0;let P=new ke(E,g);P.name="continuous-inner-screen",a.add(P);let O=.15,G=new ke(new Rn(Jd,Jd,os-.012,32,1,!1,Math.PI/2+O,Math.PI-O*2),h);G.name="hinge-spine",G.position.z=rs,a.add(G);let U=new ke(new yn(dv(r)),v);U.name="cover-screen",U.position.set(-r/2,0,-Un/2-.003),U.rotation.y=Math.PI,b.add(U);let ee=new ut;ee.name="rear-camera";let W=.46,K=.2;ee.position.set(r-.07-W/2,os/2-.08-K/2,-Un/2-.002),ee.rotation.y=Math.PI,c.add(ee);let $=.02,he=new ke(new li($d(W,K,.07),{depth:$,bevelEnabled:!0,bevelSize:.008,bevelThickness:.006,bevelSegments:3,curveSegments:12}),m);he.name="camera-plate",ee.add(he);let de=$+.006;for(let[N,X]of[[-.14,.05],[-.01,.05],[.105,.036]]){let j=new ke(new Rn(X+.012,X+.012,.012,32),u);j.rotation.x=Math.PI/2,j.position.set(N,0,de+.004),ee.add(j);let ge=new ke(new ai(X,32),_);ge.name="camera-lens",ge.position.set(N,0,de+.0105),ee.add(ge)}let q=new ke(new ai(.018,20),p);q.position.set(.185,.045,de+5e-4),ee.add(q);for(let[N,X]of[[.52,.16],[.2,.3]]){let j=new ke(new sn(.02,X,Un*.45),u);j.position.set(r+.008,N,0),c.add(j)}let ie=new ui,ce=new pe,Ce=new L,ue=null,H=e,se=t,fe=()=>{let N=se>=90;P.visible=N,y.visible=N,M.visible=N,U.visible=!N},Le=N=>{se=Math.max(0,Math.min(180,N)),l.rotation.y=Math.PI*(1-se/180),G.rotation.y=l.rotation.y/2;let X=l.rotation.y,j=Math.cos(X),ge=Math.sin(X),le=Un/2+.004-rs;for(let Re=0;Re<R.count;Re++){let xe=k[Re];xe<0?R.setXYZ(Re,xe*j+le*ge,R.getY(Re),-xe*ge+le*j+rs):R.setXYZ(Re,xe,R.getY(Re),le+rs)}R.needsUpdate=!0,E.computeBoundingBox(),E.computeBoundingSphere(),fe()},V=(N,X)=>{H=X,g.map=N,v.map=N,U.geometry.computeBoundingBox();let j=U.geometry.boundingBox,ge=U.geometry.getAttribute("uv"),le=U.geometry.getAttribute("position");for(let Re=0;Re<ge.count;Re++){let xe=(le.getX(Re)-j.min.x)/(j.max.x-j.min.x),Ue=(le.getY(Re)-j.min.y)/(j.max.y-j.min.y);ge.setXY(Re,xe,Ue)}ge.needsUpdate=!0,fe()};return Le(t),V(s,e),{root:o,orientation:a,width:r*2,height:os,innerAspect:n,setAngle:Le,setDisplay:V,screenPoint(N,X,j,ge=!1){a.updateWorldMatrix(!0,!0),j.updateMatrixWorld(!0),ce.set(N*2-1,1-X*2),ie.setFromCamera(ce,j);let le=U.visible?[U]:[y,M],Re=ie.intersectObjects(le,!1)[0];if(!Re&&!ge)return ue=null,null;let xe=Re?.object??(ue?.visible?ue:le[0]);if(!xe)return null;if(Re&&(ue=xe),Re)Ce.copy(Re.point);else{let xt=new L(0,0,1).transformDirection(xe.matrixWorld),je=new L().setFromMatrixPosition(xe.matrixWorld),rt=xt.dot(je.clone().sub(ie.ray.origin))/xt.dot(ie.ray.direction);if(!Number.isFinite(rt))return null;Ce.copy(ie.ray.direction).multiplyScalar(rt).add(ie.ray.origin)}xe.worldToLocal(Ce);let Ue=xe.geometry.boundingBox,Je=Math.max(0,Math.min(1,(Ce.x-Ue.min.x)/(Ue.max.x-Ue.min.x))),Qe=Math.max(0,Math.min(1,(Ue.max.y-Ce.y)/(Ue.max.y-Ue.min.y))),B=xe===U?Je:(xe===y?Je:1+Je)/2;return H.rotation===Math.PI?{x:1-B,y:1-Qe}:{x:B,y:Qe}},dispose(){o.traverse(N=>{N instanceof ke&&N.geometry.dispose()});for(let N of x)N.dispose()}}}var pv=450,jd=850;function Qd(s){let e=new Zs({canvas:s.canvas,alpha:!0,antialias:!0,powerPreference:"low-power"});e.outputColorSpace=wt;let t=()=>{let H=new Hi(s.source);return H.colorSpace=wt,H.minFilter=Dt,H.magFilter=Dt,H.generateMipmaps=!1,H},n=t(),i=s.source.width,r=s.source.height,o=new si,a=new Ct(32,1,.1,30);a.position.z=5.5;let l=new Zi(16777215,2.4),c=new Sn(15003135,5);c.position.set(-3,4,5);let u=new Sn(16777215,4);u.position.set(3,1,-3);let h=new Sn(10464468,2);h.position.set(-2,-2,-4),o.add(l,c,u,h);let d=null,f=Ih(d,s.source.width,s.source.height),m=s.profile??ro,_=s.foldAngle??null,p=_!==null&&m.id.startsWith("android")?0:f.rotation,g=null,S=null,v=s.model??null,x=null,b=null,w=null,y=()=>s.source.width/s.source.height,M=Oh(y())?y():Fh,C=(H,se=f)=>Kd(n,se,H,M),E=H=>{if(!w)return H;let se=Math.min(1,(performance.now()-w.startedAt)/jd),fe=se*se*(3-2*se);return w.from+(w.to-w.from)*fe},R=_!==null&&m.id.startsWith("android")?C(_):lo(n,f,m);o.add(R.root);let D=!1,k=new st,P=bl({choose:H=>wl(H,[{rotation:new st,yawLimit:Math.PI/3}]).rotation});P.setPose(k,performance.now(),!0);let O=Sl(),G=()=>globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches??!1,U={width:0,height:0,pixelRatio:1},ee={width:0,height:0,pixelRatio:0},W=null,K=(H=!1)=>{if(!U.width||!U.height)return;a.aspect=U.width/U.height;let se=new Mt(new L(-R.width/2,-R.height/2,0),new L(R.width/2,R.height/2,0));S&&b&&se.union(b),se.applyMatrix4(new ze().makeRotationZ(p));let fe=se.getSize(new L),Le=fe.x/fe.y;Le!==W&&(W=Le,s.onFramingAspect?.(Le)),R.root.updateMatrixWorld(!0),O.setBounds(new Mt().setFromObject(R.root),a.fov*Math.PI/360,a.aspect,performance.now(),H),$()},$=()=>{a.position.set(O.center.x,O.center.y,O.distance()),a.lookAt(O.center.x,O.center.y,0),a.updateProjectionMatrix()},he=()=>{R.root.quaternion.copy(P.rotation),R.orientation.rotation.z=p},de=Tl(()=>{if(!(D||!U.width||!U.height))try{(ee.width!==U.width||ee.height!==U.height||ee.pixelRatio!==U.pixelRatio)&&(e.setDrawingBufferSize(U.width,U.height,U.pixelRatio),ee=U);let H=performance.now();if(P.advance(H,G())&&(he(),K(G())),g){let se=Math.min(1,(H-g.startedAt)/pv),fe=se*se*(3-2*se);p=g.from+(g.to-g.from)*fe,se===1&&(g=null),he(),K(G())}if(w&&"setAngle"in R){let se=Math.min(1,(H-w.startedAt)/jd),fe=se*se*(3-2*se);R.setAngle(w.from+(w.to-w.from)*fe),se===1&&(w=null),K(G())}O.advance(H,G()),$(),e.render(o,a),(P.needsFrame()||O.needsFrame()||g||w)&&de.invalidate()}catch{s.onUnavailable()}}),q=(H=m)=>{let se=Ih(d,s.source.width,s.source.height),fe=i!==s.source.width||r!==s.source.height;if(fe||H!==m||se.aspect!==f.aspect||se.rawLandscape!==f.rawLandscape||se.rotation!==f.rotation){if(fe){let N=n;n=t(),i=s.source.width,r=s.source.height,R.setDisplay(n,se),N.dispose()}let Le=y(),V=Oh(Le)&&Le!==M;if(V&&(M=Le),!S&&"setAngle"in R&&V){let N=E(_??180);o.remove(R.root),R.dispose(),R=C(N,se),o.add(R.root)}else!S&&!("setAngle"in R)&&(H!==m||se.aspect!==f.aspect)?(o.remove(R.root),R.dispose(),R=lo(n,se,H),o.add(R.root)):R.setDisplay(n,se);if(se.rotation!==f.rotation)if(H.id.startsWith("android")&&!("setAngle"in R)&&!G()){let N=Math.atan2(Math.sin(se.rotation-p),Math.cos(se.rotation-p));g={from:p,to:p+N,startedAt:performance.now()}}else g=null,p="setAngle"in R?0:se.rotation;f=se,m=H,he(),K(!g)}he()},ie=ho({load:co,onError:s.onModelError,install(H){let se=D?null:H?zd(H.asset,n,f):_!==null&&m.id.startsWith("android")?C(_):lo(n,f,m);w=null,o.remove(R.root),R.dispose(),S=H,se&&(R=se,o.add(R.root),x&&(S?R.orientation.add(x.asset):x.asset.removeFromParent()),he(),K(!0),de.invalidate(),H&&s.onModelReady?.())}}),ce=ho({load:co,onError:s.onModelError,install(H){let se=H?new Mt().setFromObject(H.asset):null;if(se&&(se.isEmpty()||![...se.min.toArray(),...se.max.toArray()].every(Number.isFinite)))throw new Error("Device accessory has invalid bounds");x?.asset.removeFromParent(),x=H,b=se,!D&&(S&&x&&R.orientation.add(x.asset),K(!0),de.invalidate())}}),Ce=H=>{ce.set(H?.modelId===v?.id?H:null)},ue=H=>{H.preventDefault(),s.onUnavailable()};return s.canvas.addEventListener("webglcontextlost",ue),he(),ie.set(s.model??null),Ce(s.accessory??null),{setModel(H){(H?.id!==v?.id||H?.url!==v?.url)&&ce.set(null),v=H,ie.set(H)},setAccessory:Ce,setFoldAngle(H){if(D||H===_)return;let se=_;if(_=H,!S){if(H===null||!("setAngle"in R))o.remove(R.root),R.dispose(),R=H===null?lo(n,f,m):C(H),o.add(R.root),g=null,p=H===null?f.rotation:0,w=null,he(),K(!0);else{let fe=E(se??H);G()?(w=null,R.setAngle(H),K(!0)):w={from:fe,to:H,startedAt:performance.now()}}de.invalidate()}},frameUpdated(){D||(q(),n.needsUpdate=!0,de.invalidate())},setScreen(H,se=m){D||(d=H,q(se),de.invalidate())},resize(H,se,fe){if(D||![H,se,fe].every(Number.isFinite)||H<=0||se<=0)return;let Le=Math.min(2,Math.max(1,fe));U.width===H&&U.height===se&&U.pixelRatio===Le||(U={width:H,height:se,pixelRatio:Le},K(!0),de.invalidate())},screenPoint(H,se,fe=!1){return D?null:(he(),R.screenPoint(H,se,a,fe))},orbit(H,se){D||(P.orbit(H*U.width,se*U.height,performance.now()),de.invalidate())},setInteractionActive(H,se){if(D)return;let fe=performance.now();se==="orbit"?P.dragActive(H,fe):(P.hold(H,fe),O.hold(H,fe)),de.invalidate()},resetPose(){D||(P.reset(k,performance.now()),de.invalidate())},dispose(){if(D)return;D=!0,de.dispose();let H=S!==null;ce.dispose(),ie.dispose(),s.canvas.removeEventListener("webglcontextlost",ue),o.remove(R.root),H||R.dispose(),n.dispose(),e.dispose(),e.forceContextLoss()}}}var Gh=new URLSearchParams(location.search),np=Gh.get("platform")==="android"?"android":"ios",ip=Gh.get("name")??"";document.body.style.background=Gh.get("bg")??"transparent";var Li=s=>window.ipc.postMessage(JSON.stringify(s)),mn=document.createElement("canvas");mn.style.cssText="position:fixed;inset:0;width:100%;height:100%;touch-action:none;outline:none";mn.tabIndex=0;document.body.append(mn);var Bh=(s,e)=>Object.assign(document.createElement("canvas"),{width:s,height:e}),fo=Bh(900,1950),po=kd(np,ip),zh=po?{id:po,url:`model/${po}`}:null,kh=!po,sp=!1,ep=!1,Vh=()=>{ep||!kh||!sp||(ep=!0,Li({ready:!0}))},rp={onUnavailable:()=>Li({note:"3D view unavailable"}),onModelError:s=>{Li({note:`Device model: ${String(s)}`}),kh=!0,Vh()},onModelReady:()=>{kh=!0,Vh()}},op={1:Bh(1398,2034),3:Bh(2007,2853)},Di=po==="iphone-duo"&&zh?Wd({canvas:mn,sources:op,model:zh,...rp,onOrientationRequested:s=>Li({hold:s})}):null,Cl=Di?null:Qd({canvas:mn,source:fo,...rp,profile:Bd({platform:np,name:ip,portraitAspect:.46}),model:zh}),mo=Di??Cl;window.shepherdReset=()=>mo.resetPose();var ap=()=>mo.resize(innerWidth,innerHeight,devicePixelRatio);addEventListener("resize",ap);ap();var Pl=null,tp=null,Il=s=>JSON.stringify(s);function mv(s,e,t){let n=e===3,i=op[e],[r,o]=n?[s.height,s.width]:[s.width,s.height];(i.width!==r||i.height!==o)&&Object.assign(i,{width:r,height:o});let a=i.getContext("2d");a.save(),n&&(a.translate(0,o),a.rotate(-Math.PI/2)),a.drawImage(s,0,0),a.restore();let l={width:r,height:o,orientation:t?.orientation??"portrait",screenId:e,supportsHingeAngle:!0,supportsPhysicalOrientation:!1,hingeAngle:t?.angle??(n?180:0),hingePose:t?.stand??null};Il(l)!==Il(Pl)&&Di.setScreen(Pl=l),Di.frameUpdated(e)}async function gv(){let s=0;for(;;)try{let e=await fetch(`frame?after=${s}`);if(e.status!==200)continue;s=Number(e.headers.get("x-seq")??s);let t=JSON.parse(e.headers.get("x-fold")??"null"),n=await createImageBitmap(await e.blob());if(Di)mv(n,e.headers.get("x-screen")==="1"?1:3,t);else{(fo.width!==n.width||fo.height!==n.height)&&Object.assign(fo,{width:n.width,height:n.height});let i=e.headers.get("x-orientation")??"portrait",r=fo.getContext("2d");r.save(),i==="portrait_upside_down"&&(r.translate(n.width,n.height),r.rotate(Math.PI)),r.drawImage(n,0,0),r.restore();let o={width:n.width,height:n.height,orientation:i};Il(o)!==Il(tp)&&Cl.setScreen(tp=o),Cl.frameUpdated(),Cl.setFoldAngle(t&&!t.duo?t.angle:null)}n.close(),sp=!0,Vh()}catch{await new Promise(e=>setTimeout(e,500))}}gv();var go=qd({screenPoint:(s,e)=>mo.screenPoint(s.x,s.y,e),touch:(s,e)=>Li({touch:s,x:e.x,y:e.y}),orbit:mo.orbit,zoomBy:()=>{},onInteractionActive:mo.setInteractionActive}),Rl=Di?eu({angle:()=>Pl?.hingeAngle??180,contains:(s,e)=>!!Pl&&Di.beginHinge(s,e),change:s=>{Di.setHingePreview(s,!0),s!==null&&Li({hinge:s})}}):void 0,lp=s=>{let e=mn.getBoundingClientRect();return{x:(s.clientX-e.left)/e.width,y:(s.clientY-e.top)/e.height}};mn.addEventListener("pointerdown",s=>{mn.focus(),mn.setPointerCapture(s.pointerId),go.begin(s.pointerId,lp(s),s.button!==0)});mn.addEventListener("pointermove",s=>go.move(s.pointerId,lp(s)));for(let s of["pointerup","pointercancel","lostpointercapture"])mn.addEventListener(s,e=>go.end(e.pointerId));mn.addEventListener("contextmenu",s=>s.preventDefault());Yd(mn,go,Rl&&{begin(s,e){return go.end(),Rl.begin(s,e)},move:Rl.move,end:Rl.end});var _v={Enter:"enter",Backspace:"backspace",Delete:"delete",Tab:"tab",Escape:"escape",ArrowLeft:"left",ArrowRight:"right",ArrowUp:"up",ArrowDown:"down",Home:"home",End:"end",PageUp:"pageup",PageDown:"pagedown"};addEventListener("keydown",s=>{if(s.metaKey||s.ctrlKey)return;let e=_v[s.key];if(e)Li({key:e});else if(s.key.length===1)Li({text:s.key});else return;s.preventDefault()});})();
