#[allow(unused_variables)]
#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
pub mod predefined_functions;
use crate::{
    executable::{PPEExpr, VariableValue},
    Res,
};
pub use predefined_functions::*;

type PredefFunc = fn(vm: &mut crate::vm::VirtualMachine, arguments: &[PPEExpr]) -> Res<VariableValue>;

pub static FUNCTION_TABLE: [PredefFunc; 294] = [
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::invalid,
    predefined_functions::len,
    predefined_functions::lower,
    predefined_functions::upper,
    predefined_functions::mid,
    predefined_functions::left,
    predefined_functions::right,
    predefined_functions::space,
    predefined_functions::ferr,
    predefined_functions::chr,
    predefined_functions::asc,
    predefined_functions::instr,
    predefined_functions::abort,
    predefined_functions::ltrim,
    predefined_functions::rtrim,
    predefined_functions::trim,
    predefined_functions::random,
    predefined_functions::date,
    predefined_functions::time,
    predefined_functions::u_name,
    predefined_functions::u_ldate,
    predefined_functions::u_ltime,
    predefined_functions::u_ldir,
    predefined_functions::u_logons,
    predefined_functions::u_ful,
    predefined_functions::u_fdl,
    predefined_functions::u_bdlday,
    predefined_functions::u_timeon,
    predefined_functions::u_bdl,
    predefined_functions::u_bul,
    predefined_functions::year,
    predefined_functions::month,
    predefined_functions::day,
    predefined_functions::dow,
    predefined_functions::hour,
    predefined_functions::min,
    predefined_functions::sec,
    predefined_functions::timeap,
    predefined_functions::ver,
    predefined_functions::nochar,
    predefined_functions::yeschar,
    predefined_functions::stripatx,
    predefined_functions::replace,
    predefined_functions::strip,
    predefined_functions::inkey,
    predefined_functions::tostring,
    predefined_functions::mask_pwd,
    predefined_functions::mask_alpha,
    predefined_functions::mask_num,
    predefined_functions::mask_alnum,
    predefined_functions::mask_file,
    predefined_functions::mask_path,
    predefined_functions::mask_ascii,
    predefined_functions::curconf,
    predefined_functions::pcbdat,
    predefined_functions::ppepath,
    predefined_functions::valdate,
    predefined_functions::valtime,
    predefined_functions::u_msgrd,
    predefined_functions::u_msgwr,
    predefined_functions::pcbnode,
    predefined_functions::readline,
    predefined_functions::sysopsec,
    predefined_functions::onlocal,
    predefined_functions::un_stat,
    predefined_functions::un_name,
    predefined_functions::un_city,
    predefined_functions::un_oper,
    predefined_functions::cursec,
    predefined_functions::gettoken,
    predefined_functions::minleft,
    predefined_functions::minon,
    predefined_functions::getenv,
    predefined_functions::callid,
    predefined_functions::regal,
    predefined_functions::regah,
    predefined_functions::regbl,
    predefined_functions::regbh,
    predefined_functions::regcl,
    predefined_functions::regch,
    predefined_functions::regdl,
    predefined_functions::regdh,
    predefined_functions::regax,
    predefined_functions::regbx,
    predefined_functions::regcx,
    predefined_functions::regdx,
    predefined_functions::regsi,
    predefined_functions::regdi,
    predefined_functions::regf,
    predefined_functions::regcf,
    predefined_functions::regds,
    predefined_functions::reges,
    predefined_functions::b2w,
    predefined_functions::peekb,
    predefined_functions::peekw,
    predefined_functions::mkaddr,
    predefined_functions::exist,
    predefined_functions::i2s,
    predefined_functions::s2i,
    predefined_functions::carrier,
    predefined_functions::tokenstr,
    predefined_functions::cdon,
    predefined_functions::langext,
    predefined_functions::ansion,
    predefined_functions::valcc,
    predefined_functions::fmtcc,
    predefined_functions::cctype,
    predefined_functions::getx,
    predefined_functions::gety,
    predefined_functions::band,
    predefined_functions::bor,
    predefined_functions::bxor,
    predefined_functions::bnot,
    predefined_functions::u_pwdhist,
    predefined_functions::u_pwdlc,
    predefined_functions::u_pwdtc,
    predefined_functions::u_stat,
    predefined_functions::defcolor,
    predefined_functions::abs,
    predefined_functions::grafmode,
    predefined_functions::psa,
    predefined_functions::fileinf,
    predefined_functions::ppename,
    predefined_functions::mkdate,
    predefined_functions::curcolor,
    predefined_functions::kinkey,
    predefined_functions::minkey,
    predefined_functions::maxnode,
    predefined_functions::slpath,
    predefined_functions::helppath,
    predefined_functions::temppath,
    predefined_functions::modem,
    predefined_functions::loggedon,
    predefined_functions::callnum,
    predefined_functions::mgetbyte,
    predefined_functions::tokcount,
    predefined_functions::u_recnum,
    predefined_functions::u_inconf,
    predefined_functions::peekdw,
    predefined_functions::dbglevel,
    predefined_functions::scrtext,
    predefined_functions::showstat,
    predefined_functions::pagestat,
    predefined_functions::replacestr,
    predefined_functions::stripstr,
    predefined_functions::tobigstr,
    predefined_functions::toboolean,
    predefined_functions::tobyte,
    predefined_functions::todate,
    predefined_functions::todreal,
    predefined_functions::toedate,
    predefined_functions::tointeger,
    predefined_functions::tomoney,
    predefined_functions::toreal,
    predefined_functions::tosbyte,
    predefined_functions::tosword,
    predefined_functions::totime,
    predefined_functions::tounsigned,
    predefined_functions::toword,
    predefined_functions::mixed,
    predefined_functions::alias,
    predefined_functions::confreg,
    predefined_functions::confexp,
    predefined_functions::confsel,
    predefined_functions::confsys,
    predefined_functions::confmw,
    predefined_functions::lprinted,
    predefined_functions::isnonstop,
    predefined_functions::errcorrect,
    predefined_functions::confalias,
    predefined_functions::useralias,
    predefined_functions::curuser,
    predefined_functions::u_lmr,
    predefined_functions::chatstat,
    predefined_functions::defans,
    predefined_functions::lastans,
    predefined_functions::meganum,
    predefined_functions::evttimeadj,
    predefined_functions::isbitset,
    predefined_functions::fmtreal,
    predefined_functions::flagcnt,
    predefined_functions::kbdbufsize,
    predefined_functions::pplbufsize,
    predefined_functions::kbdfilusued,
    predefined_functions::lomsgnum,
    predefined_functions::himsgnum,
    predefined_functions::drivespace,
    predefined_functions::outbytes,
    predefined_functions::hiconfnum,
    predefined_functions::inbytes,
    predefined_functions::crc32,
    predefined_functions::pcbmac,
    predefined_functions::actmsgnum,
    predefined_functions::stackleft,
    predefined_functions::stackerr,
    predefined_functions::dgetalias,
    predefined_functions::dbof,
    predefined_functions::dchanged,
    predefined_functions::ddecimals,
    predefined_functions::ddeleted,
    predefined_functions::deof,
    predefined_functions::derr,
    predefined_functions::dfields,
    predefined_functions::dlength,
    predefined_functions::dname,
    predefined_functions::dreccount,
    predefined_functions::drecno,
    predefined_functions::dtype,
    predefined_functions::fnext,
    predefined_functions::dnext,
    predefined_functions::toddate,
    predefined_functions::dcloseall,
    predefined_functions::dopen,
    predefined_functions::dclose,
    predefined_functions::dsetalias,
    predefined_functions::dpack,
    predefined_functions::dlockf,
    predefined_functions::dlock,
    predefined_functions::dlockr,
    predefined_functions::dunlock,
    predefined_functions::dnopen,
    predefined_functions::dnclose,
    predefined_functions::dncloseall,
    predefined_functions::dnew,
    predefined_functions::dadd,
    predefined_functions::dappend,
    predefined_functions::dtop,
    predefined_functions::dgo,
    predefined_functions::dbottom,
    predefined_functions::dskip,
    predefined_functions::dblank,
    predefined_functions::ddelete,
    predefined_functions::drecall,
    predefined_functions::dtag,
    predefined_functions::dseek,
    predefined_functions::dfblank,
    predefined_functions::dget,
    predefined_functions::dput,
    predefined_functions::dfcopy,
    predefined_functions::dselect,
    predefined_functions::dchkstat,
    predefined_functions::pcbaccount,
    predefined_functions::pcbaccstat,
    predefined_functions::derrmsg,
    predefined_functions::account,
    predefined_functions::scanmsghdr,
    predefined_functions::checkrip,
    predefined_functions::ripver,
    predefined_functions::qwklimits,
    predefined_functions::findfirst,
    predefined_functions::findnext,
    predefined_functions::uselmrs,
    predefined_functions::confinfo,
    predefined_functions::tinkey,
    predefined_functions::cwd,
    predefined_functions::instrr,
    predefined_functions::fdordaka,
    predefined_functions::fdordorg,
    predefined_functions::fdordarea,
    predefined_functions::fdoqrd,
    predefined_functions::getdrive,
    predefined_functions::setdrive,
    predefined_functions::bs2i,
    predefined_functions::bd2i,
    predefined_functions::i2bs,
    predefined_functions::i2bd,
    predefined_functions::ftell,
    predefined_functions::os,
    predefined_functions::shortdesc,
    predefined_functions::getbankbal,
    predefined_functions::getmsghdr,
    predefined_functions::setmsghdr,
    // ALIASES
    predefined_functions::tostring,
    predefined_functions::tosword,
    predefined_functions::tounsigned,
];
