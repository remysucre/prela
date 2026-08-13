-- | Small TPC-H reference results embedded in the benchmark executable.
--
-- Larger oracle results remain external files to keep compilation reasonable;
-- "Main" chooses between these values and those files by query number.
module TPCH.Oracles (inlineOracles) where

import Data.List (intercalate)

-- | Join oracle rows using the same no-final-newline format as query renderers.
joinLines :: [String] -> String
joinLines = intercalate "\n"

-- | Reference output for queries whose complete results are compact.
inlineOracles :: [(String, String)]
inlineOracles =
  [ ("1", joinLines
      [ "A|F|37734107.00|56586554400.73|53758257134.87|55909065222.83|25.52|38273.13|0.05|1478493"
      , "N|F|991417.00|1487504710.38|1413082168.05|1469649223.19|25.52|38284.47|0.05|38854"
      , "N|O|74476040.00|111701729697.74|106118230307.61|110367043872.49|25.50|38249.12|0.05|2920374"
      , "R|F|37719753.00|56568041380.90|53741292684.60|55889619119.83|25.51|38250.85|0.05|1478870"
      ])
  , ("3", joinLines
      [ "2456423|406181.01|1995-03-05|0"
      , "3459808|405838.70|1995-03-04|0"
      , "492164|390324.06|1995-02-19|0"
      , "1188320|384537.94|1995-03-09|0"
      , "2435712|378673.06|1995-02-26|0"
      , "4878020|378376.80|1995-03-12|0"
      , "5521732|375153.92|1995-03-13|0"
      , "2628192|373133.31|1995-02-22|0"
      , "993600|371407.46|1995-03-05|0"
      , "2300070|367371.15|1995-03-13|0"
      ])
  , ("4", joinLines
      [ "1-URGENT|10594"
      , "2-HIGH|10476"
      , "3-MEDIUM|10410"
      , "4-NOT SPECIFIED|10556"
      , "5-LOW|10487"
      ])
  , ("5", joinLines
      [ "INDONESIA|55502041.17"
      , "VIETNAM|55295087.00"
      , "CHINA|53724494.26"
      , "INDIA|52035512.00"
      , "JAPAN|45410175.70"
      ])
  , ("6", "123141078.23")
  , ("10", joinLines
      [ "57040|Customer#000057040|734235.25|632.87|JAPAN|nICtsILWBB|22-895-641-3466|ep. blithely regular foxes promise slyly furiously ironic depend"
      , "143347|Customer#000143347|721002.69|2557.47|EGYPT|,Q9Ml3w0gvX|14-742-935-3718|endencies sleep. slyly express deposits nag carefully around the even tithes. slyly regular "
      , "60838|Customer#000060838|679127.31|2454.77|BRAZIL|VWmQhWweqj5hFpcvhGFBeOY9hJ4m|12-913-494-9813|tes. final instructions nag quickly according to"
      , "101998|Customer#000101998|637029.57|3790.89|UNITED KINGDOM|0,ORojfDdyMca2E2H|33-593-865-6378|ost carefully. slyly regular packages cajole about the blithely final ideas. permanently daring deposit"
      , "125341|Customer#000125341|633508.09|4983.51|GERMANY|9YRcnoUPOM7Sa8xymhsDHdQg|17-582-695-5962|ly furiously brave packages. quickly regular dugouts kindle furiously carefully bold theodolites. "
      , "25501|Customer#000025501|620269.78|7725.04|ETHIOPIA|sr4VVVe3xCJQ2oo2QEhi19D,pXqo6kOGaSn2|15-874-808-6793|y ironic foxes hinder according to the furiously permanent dolphins. pending ideas integrate blithely from "
      , "115831|Customer#000115831|596423.87|5098.10|FRANCE|AlMpPnmtGrOFrDMUs5VLo EIA,Cg,Rw5TBuBoKiO|16-715-386-3788|unts nag carefully final packages. express theodolites are regular ac"
      , "84223|Customer#000084223|594998.02|528.65|UNITED KINGDOM|Eq51o UpQ4RBr  fYTdrZApDsPV4pQyuPq|33-442-824-8191|longside of the slyly final deposits. blithely final platelets about the blithely i"
      , "54289|Customer#000054289|585603.39|5583.02|IRAN|x3ouCpz6,pRNVhajr0CCQG1|20-834-292-4707| cajole furiously after the quickly unusual fo"
      , "39922|Customer#000039922|584878.11|7321.11|GERMANY|2KtWzW,FYkhdWBfobp6SFXWYKjvU9|17-147-757-8036|ironic deposits sublate furiously. carefully regular theodolites along the b"
      , "6226|Customer#000006226|576783.76|2230.09|UNITED KINGDOM|TKbxS1dbkGMtaa,KOi26lbip4P0tPbWK0|33-657-701-3391|nal packages are alongside of the quickly bold deposits. carefully "
      , "922|Customer#000000922|576767.53|3869.25|GERMANY|rsR9lRxyTdHbDOVt8nYbwjK5vAWH9sB|17-945-916-9648|cuses cajole carefully regular idea"
      , "147946|Customer#000147946|576455.13|2030.13|ALGERIA|Jqdt1kHAJtuTqHQK,B7 3tJh|10-886-956-3143|ly pending platelets. ironic requests haggle alongside of the furiou"
      , "115640|Customer#000115640|569341.19|6436.10|ARGENTINA|6yKLIRRAirUmBjKNO6Z3|11-411-543-4901|ffily ironic deposits. blithely specia"
      , "73606|Customer#000073606|568656.86|1785.67|JAPAN|vx9,7ACVtoKnLcoAHGNYDF|22-437-653-6966|uests cajole according to the foxe"
      , "110246|Customer#000110246|566842.98|7763.35|VIETNAM|UgsLFL3rendATzcHi|31-943-426-9837|ow carefully. blithely careful packages hag"
      , "142549|Customer#000142549|563537.24|5085.99|INDONESIA|pJAmChWXct HNjPzgoBUOgAHduwwIR|19-955-562-2398|. slyly bold packages nag quickly against the unusual deposits. express asymptotes detect furiously pending, eve"
      , "146149|Customer#000146149|557254.99|1791.55|ROMANIA| STLwtlaB6|29-744-164-6487|nic, special instructions. multipliers run carefully blithely iro"
      , "52528|Customer#000052528|556397.35|551.79|ARGENTINA|elsyt8c9Z,7ch|11-208-192-3205|olphins. blithely silent platelets affix carefully even platelets. ca"
      , "23431|Customer#000023431|554269.54|3381.86|ROMANIA|kKI5,CJAJQjQRQtOdCiFQ|29-915-458-2654|the final sentiments. carefully ironic packages"
      ])
  , ("12", joinLines
      [ "MAIL|6202|9324"
      , "SHIP|6200|9262"
      ])
  , ("14", "16.38")
  ]
