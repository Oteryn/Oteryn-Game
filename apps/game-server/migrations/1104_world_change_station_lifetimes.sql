-- Thornfire source stations extend the existing craft, terrain and Item lifetime owners.
-- Exact authored placement tails retain their source entry ordinal; no inferred rectangle.
CREATE FUNCTION game_thornfire_source_station(p_variant INT,p_placement BYTEA,p_before BIGINT)
RETURNS BOOLEAN LANGUAGE SQL IMMUTABLE STRICT SET search_path=pg_catalog AS $$
 SELECT octet_length(p_placement)=40 AND EXISTS (
  SELECT 1 FROM (VALUES
(0,decode('000081717d920700','hex'),12721),
(0,decode('000081727d920700','hex'),12721),
(0,decode('000081737d920700','hex'),12721),
(0,decode('000081747d920700','hex'),12721),
(0,decode('000081757d920700','hex'),12721),
(0,decode('000081767d920700','hex'),12721),
(0,decode('000081777d920700','hex'),12721),
(0,decode('000081787d920700','hex'),12721),
(0,decode('000081717d930700','hex'),12721),
(0,decode('000081727d930700','hex'),12721),
(0,decode('000081737d930700','hex'),12721),
(0,decode('000081747d930700','hex'),12721),
(0,decode('000081757d930700','hex'),12721),
(0,decode('000081767d930700','hex'),12721),
(0,decode('000081777d930700','hex'),12721),
(0,decode('000081787d930700','hex'),12721),
(0,decode('000081717d940700','hex'),12721),
(0,decode('000081727d940700','hex'),12721),
(0,decode('000081737d940700','hex'),12721),
(0,decode('000081747d940700','hex'),12721),
(0,decode('000081757d940700','hex'),12721),
(0,decode('000081767d940700','hex'),12721),
(0,decode('000081777d940700','hex'),12721),
(0,decode('000081787d940700','hex'),12721),
(0,decode('000081717d950700','hex'),12721),
(0,decode('000081727d950700','hex'),12721),
(0,decode('000081737d950700','hex'),12721),
(0,decode('000081747d950700','hex'),12721),
(0,decode('000081757d950700','hex'),12721),
(0,decode('000081767d950700','hex'),12721),
(0,decode('000081777d950700','hex'),12721),
(0,decode('000081787d950700','hex'),12721),
(0,decode('000081717d960700','hex'),12721),
(0,decode('000081727d960700','hex'),12721),
(0,decode('000081737d960700','hex'),12721),
(0,decode('000081747d960700','hex'),12721),
(0,decode('000081757d960700','hex'),12721),
(0,decode('000081767d960700','hex'),12721),
(0,decode('000081777d960700','hex'),12721),
(0,decode('000081787d960700','hex'),12721),
(0,decode('000081717d970700','hex'),12721),
(0,decode('000081727d970700','hex'),12721),
(0,decode('000081737d970700','hex'),12721),
(0,decode('000081747d970700','hex'),12721),
(0,decode('000081757d970700','hex'),12721),
(0,decode('000081767d970700','hex'),12721),
(0,decode('000081777d970700','hex'),12721),
(0,decode('000081787d970700','hex'),12721),
(0,decode('000081717d980700','hex'),12721),
(0,decode('000081727d980700','hex'),12721),
(0,decode('000081737d980700','hex'),12721),
(0,decode('000081747d980700','hex'),12721),
(0,decode('000081757d980700','hex'),12721),
(0,decode('000081767d980700','hex'),12721),
(0,decode('000081777d980700','hex'),12721),
(0,decode('000081787d980700','hex'),12721),
(0,decode('000081327da60202','hex'),5063),
(0,decode('000081357da60202','hex'),5063),
(0,decode('0000812e7da90202','hex'),5064),
(0,decode('000081387da90202','hex'),5065),
(0,decode('0000812f7dac0202','hex'),5063),
(0,decode('000081327daf0202','hex'),5063),
(0,decode('000081487d880301','hex'),5065),
(0,decode('000081437d8a0301','hex'),5065),
(0,decode('0000814f7d8b0302','hex'),5065),
(0,decode('000081477d8c0302','hex'),5065),
(0,decode('000081427d8d0302','hex'),5062),
(0,decode('000081427d8d0303','hex'),5065),
(0,decode('000081537d8d0302','hex'),5067),
(0,decode('000081347da50302','hex'),5065),
(0,decode('000081327da60302','hex'),5063),
(0,decode('0000812f7da90304','hex'),5063),
(0,decode('000081397da90302','hex'),5063),
(0,decode('0000812f7dac0302','hex'),5063),
(0,decode('000081317daf0302','hex'),5064),
(0,decode('000081377daf0301','hex'),5065),
(0,decode('000081387daf0301','hex'),5064),
(0,decode('000081357db00303','hex'),5063),
(0,decode('0000813e7dbc0301','hex'),5065),
(0,decode('0000813f7dbc0302','hex'),5063),
(0,decode('0000813c7dbd0302','hex'),5061),
(0,decode('0000813d7dbd0302','hex'),5064),
(0,decode('0000813d7dbd0303','hex'),5062),
(0,decode('0000813d7dbd0304','hex'),5065),
(0,decode('000081417dbf0302','hex'),5062),
(0,decode('0000813c7dc00302','hex'),5063),
(0,decode('0000813d7dc00302','hex'),5066),
(0,decode('000081407dc00302','hex'),5065),
(0,decode('0000813d7dc10302','hex'),5067),
(0,decode('000081327d870402','hex'),5063),
(0,decode('000081357d870401','hex'),5065),
(0,decode('000081437d880402','hex'),5063),
(0,decode('0000814f7d880402','hex'),5063),
(0,decode('000081517d880402','hex'),5063),
(0,decode('000081327d8a0402','hex'),5065),
(0,decode('000081377d8a0402','hex'),5063),
(0,decode('000081437d8a0401','hex'),5065),
(0,decode('000081537d8a0402','hex'),5067),
(0,decode('0000814f7d8b0402','hex'),5065),
(0,decode('000081387d8c0401','hex'),5065),
(0,decode('000081427d8d0403','hex'),5063),
(0,decode('000081457d8d0402','hex'),5064),
(0,decode('000081537d8d0402','hex'),5067),
(0,decode('000081537d8d0403','hex'),5065),
(0,decode('000081237d9b0401','hex'),5066),
(0,decode('000081277d9b0401','hex'),5063),
(0,decode('000081297d9b0401','hex'),5064),
(0,decode('0000812b7d9c0401','hex'),5067),
(0,decode('0000812d7d9c0402','hex'),5063),
(0,decode('000081207d9e0401','hex'),5063),
(0,decode('000081217da00401','hex'),5065),
(0,decode('0000812d7da10400','hex'),5065),
(0,decode('000081267da20401','hex'),5065),
(0,decode('0000812a7da30401','hex'),5063),
(0,decode('000081247da40400','hex'),5067),
(0,decode('000081327da60404','hex'),5063),
(0,decode('0000812f7da90402','hex'),5063),
(0,decode('000081397da90402','hex'),5063),
(0,decode('0000812f7dac0402','hex'),5063),
(0,decode('000081317dae0402','hex'),5063),
(0,decode('000081357db00402','hex'),5063),
(0,decode('0000813e7dbc0402','hex'),5066),
(0,decode('0000813f7dbc0402','hex'),5064),
(0,decode('000081407dbe0401','hex'),5065),
(0,decode('000081417dbe0402','hex'),5066),
(0,decode('0000813c7dbf0402','hex'),5067),
(0,decode('000081417dbf0402','hex'),5063),
(0,decode('0000813d7dc00402','hex'),5062),
(0,decode('000081417dc00402','hex'),5067),
(0,decode('0000811a7dcd0402','hex'),5066),
(0,decode('0000811a7dcf0402','hex'),5066),
(0,decode('0000811e7dd10402','hex'),5063),
(0,decode('000081347d870502','hex'),5067),
(0,decode('000081437d880502','hex'),5065),
(0,decode('000081457d880502','hex'),5063),
(0,decode('0000814b7d880502','hex'),5063),
(0,decode('0000814f7d880502','hex'),5063),
(0,decode('000081327d8a0502','hex'),5067),
(0,decode('000081377d8a0502','hex'),5063),
(0,decode('000081417d8a0501','hex'),5064),
(0,decode('000081427d8a0502','hex'),5065),
(0,decode('000081537d8a0502','hex'),5063),
(0,decode('000081317d8b0501','hex'),5066),
(0,decode('0000814f7d8b0502','hex'),5063),
(0,decode('000081327d8c0502','hex'),5063),
(0,decode('000081357d8c0502','hex'),5065),
(0,decode('000081477d8d0502','hex'),5063),
(0,decode('000081217d9b0501','hex'),5063),
(0,decode('000081257d9b0501','hex'),5064),
(0,decode('000081277d9b0501','hex'),5063),
(0,decode('0000812b7d9b0501','hex'),5063),
(0,decode('0000812d7d9c0501','hex'),5065),
(0,decode('000081207d9e0501','hex'),5063),
(0,decode('0000812d7d9e0502','hex'),5065),
(0,decode('0000812d7da00501','hex'),5066),
(0,decode('0000812d7da00502','hex'),5065),
(0,decode('0000812e7da00501','hex'),5067),
(0,decode('0000812d7da10501','hex'),5063),
(0,decode('0000812d7da20501','hex'),5065),
(0,decode('000081207da30501','hex'),5064),
(0,decode('000081207da30502','hex'),5062),
(0,decode('0000812b7da30501','hex'),5067),
(0,decode('0000812c7da30501','hex'),5066),
(0,decode('0000812d7da30501','hex'),5067),
(0,decode('0000812a7da40500','hex'),5067),
(0,decode('000081337da50502','hex'),5065),
(0,decode('000081367da70502','hex'),5061),
(0,decode('000081327da80502','hex'),5063),
(0,decode('000081367dab0502','hex'),5063),
(0,decode('000081357dac0502','hex'),5063),
(0,decode('0000812e7dad0500','hex'),5067),
(0,decode('000081377dae0502','hex'),5063),
(0,decode('000081317daf0502','hex'),5064),
(0,decode('000081327daf0502','hex'),5063),
(0,decode('000081367daf0502','hex'),5067),
(0,decode('0000813e7dbb0500','hex'),5065),
(0,decode('0000813c7dbc0500','hex'),5062),
(0,decode('0000813f7dbc0502','hex'),5061),
(0,decode('000081407dbc0502','hex'),5062),
(0,decode('000081417dbd0502','hex'),5066),
(0,decode('0000813c7dbe0502','hex'),5067),
(0,decode('000081417dbe0502','hex'),5067),
(0,decode('0000813c7dbf0502','hex'),5064),
(0,decode('0000813c7dc00502','hex'),5064),
(0,decode('000081417dc00502','hex'),5065),
(0,decode('0000813d7dc10502','hex'),5063),
(0,decode('0000813f7dc10502','hex'),5064),
(0,decode('000081407dc10502','hex'),5063),
(0,decode('0000811a7dcd0502','hex'),5064),
(0,decode('0000811d7dcd0501','hex'),5065),
(0,decode('0000811e7dcd0503','hex'),5066),
(0,decode('0000811a7dcf0502','hex'),5063),
(0,decode('0000811a7dd10502','hex'),5066),
(0,decode('0000811e7dd10502','hex'),5063),
(0,decode('0000811c7dd20502','hex'),5066),
(0,decode('000081327d870603','hex'),5063),
(0,decode('000081347d870603','hex'),5063),
(0,decode('000081377d870603','hex'),5065),
(0,decode('000081397d880603','hex'),5063),
(0,decode('0000813c7d880601','hex'),5065),
(0,decode('0000813f7d880602','hex'),5064),
(0,decode('000081457d880602','hex'),5063),
(0,decode('000081497d880602','hex'),5065),
(0,decode('0000814f7d880602','hex'),5063),
(0,decode('000081517d880602','hex'),5061),
(0,decode('0000814e7d890601','hex'),5065),
(0,decode('000081427d8a0602','hex'),5063),
(0,decode('000081477d8a0602','hex'),5063),
(0,decode('000081537d8a0603','hex'),5063),
(0,decode('000081397d8b0605','hex'),5063),
(0,decode('000081487d8c0601','hex'),5065),
(0,decode('000081317d8d0601','hex'),5063),
(0,decode('000081367d8d0603','hex'),5063),
(0,decode('000081427d8d0602','hex'),5063),
(0,decode('000081477d8d0602','hex'),5063),
(0,decode('000081417d8e0602','hex'),5066),
(0,decode('000081447d8e0602','hex'),5066),
(0,decode('000081317d910601','hex'),5061),
(0,decode('0000815a7d920601','hex'),5065),
(0,decode('000081317d930601','hex'),5064),
(0,decode('000081377d930602','hex'),5065),
(0,decode('000081317d950601','hex'),5061),
(0,decode('0000813f7d970601','hex'),5062),
(0,decode('000081327d990601','hex'),5065),
(0,decode('000081547d9a0602','hex'),5064),
(0,decode('000081217d9b0601','hex'),5064),
(0,decode('000081257d9b0602','hex'),5063),
(0,decode('0000812b7d9b0601','hex'),5063),
(0,decode('000081207d9c0601','hex'),5065),
(0,decode('0000812d7d9f0602','hex'),5063),
(0,decode('000081207da00601','hex'),5063),
(0,decode('0000812e7da20601','hex'),5064),
(0,decode('000081257da30602','hex'),5063),
(0,decode('0000814e7da30601','hex'),5067),
(0,decode('000081237da40601','hex'),5063),
(0,decode('0000812a7da40601','hex'),5066),
(0,decode('0000814f7da40602','hex'),5065),
(0,decode('0000812a7da50601','hex'),5063),
(0,decode('000081247da60601','hex'),5065),
(0,decode('0000812a7da60601','hex'),5064),
(0,decode('000081357da60602','hex'),5065),
(0,decode('0000812a7da70601','hex'),5066),
(0,decode('000081367da70602','hex'),5064),
(0,decode('000081237da80601','hex'),5063),
(0,decode('0000812a7da80601','hex'),5062),
(0,decode('000081377da80602','hex'),5063),
(0,decode('000081447da80601','hex'),5066),
(0,decode('000081457da80602','hex'),5064),
(0,decode('0000812a7da90601','hex'),5062),
(0,decode('0000812f7da90603','hex'),5063),
(0,decode('0000812a7daa0601','hex'),5067),
(0,decode('0000812a7dab0601','hex'),5066),
(0,decode('0000812e7dac0602','hex'),5064),
(0,decode('0000812f7dac0602','hex'),5063),
(0,decode('0000811e7dad0601','hex'),5065),
(0,decode('0000811d7dae0600','hex'),5066),
(0,decode('000081317dae0602','hex'),5063),
(0,decode('0000811f7daf0601','hex'),5065),
(0,decode('000081207db00601','hex'),5067),
(0,decode('000081327db00602','hex'),5063),
(0,decode('0000811e7db10601','hex'),5066),
(0,decode('000081327db10601','hex'),5064),
(0,decode('0000811e7db30601','hex'),5063),
(0,decode('0000811f7db40601','hex'),5065),
(0,decode('0000811c7db80601','hex'),5063),
(0,decode('000081307db80603','hex'),5064),
(0,decode('000081377db80602','hex'),5064),
(0,decode('000081297dba0602','hex'),5063),
(0,decode('0000811e7dbc0601','hex'),5066),
(0,decode('0000813e7dbc0602','hex'),5064),
(0,decode('000081417dbd0602','hex'),5066),
(0,decode('000081417dbe0602','hex'),5062),
(0,decode('000081247dbf0601','hex'),5066),
(0,decode('000081417dbf0602','hex'),5067),
(0,decode('0000813c7dc00602','hex'),5061),
(0,decode('000081267dc10603','hex'),5063),
(0,decode('0000813d7dc10602','hex'),5064),
(0,decode('0000813f7dc10602','hex'),5064),
(0,decode('0000811b7dc20602','hex'),5064),
(0,decode('000081207dc40602','hex'),5065),
(0,decode('000081287dcc0602','hex'),5063),
(0,decode('0000811a7dcd0602','hex'),5064),
(0,decode('0000811a7dcf0602','hex'),5063),
(0,decode('0000811c7dd10602','hex'),5063),
(0,decode('000081217dd10603','hex'),5063),
(0,decode('0000813a7d890703','hex'),5061),
(0,decode('0000813c7d890702','hex'),5064),
(0,decode('0000813c7d890704','hex'),5061),
(0,decode('0000813f7d890704','hex'),5061),
(0,decode('000081407d890704','hex'),5061),
(0,decode('000081467d890703','hex'),5061),
(0,decode('000081497d890703','hex'),5061),
(0,decode('0000814a7d890702','hex'),5061),
(0,decode('0000814b7d890702','hex'),5061),
(0,decode('0000814c7d890703','hex'),5061),
(0,decode('000081507d890703','hex'),5061),
(0,decode('000081517d890703','hex'),5061),
(0,decode('000081527d890704','hex'),5061),
(0,decode('000081317d8a0703','hex'),5062),
(0,decode('0000813f7d8a0702','hex'),5064),
(0,decode('000081427d8a0703','hex'),5062),
(0,decode('000081477d8a0703','hex'),5062),
(0,decode('0000814c7d8a0701','hex'),5065),
(0,decode('0000814e7d8a0702','hex'),5062),
(0,decode('000081317d8b0702','hex'),5062),
(0,decode('000081327d8b0703','hex'),5062),
(0,decode('0000813d7d8b0701','hex'),5065),
(0,decode('0000813f7d8b0702','hex'),5064),
(0,decode('0000814d7d8b0701','hex'),5067),
(0,decode('0000814e7d8b0702','hex'),5062),
(0,decode('000081317d8c0702','hex'),5062),
(0,decode('000081407d8c0702','hex'),5067),
(0,decode('000081427d8c0702','hex'),5065),
(0,decode('000081427d8c0703','hex'),5062),
(0,decode('000081477d8c0702','hex'),5062),
(0,decode('0000814d7d8c0701','hex'),5066),
(0,decode('000081317d8d0702','hex'),5062),
(0,decode('000081427d8d0703','hex'),5061),
(0,decode('000081437d8d0702','hex'),5061),
(0,decode('000081437d8d0703','hex'),5063),
(0,decode('000081467d8d0703','hex'),5061),
(0,decode('000081477d8d0702','hex'),5065),
(0,decode('0000814e7d8d0703','hex'),5063),
(0,decode('000081517d8d0703','hex'),5061),
(0,decode('000081527d8d0703','hex'),5061),
(0,decode('000081537d8d0703','hex'),5061),
(0,decode('000081317d8e0702','hex'),5062),
(0,decode('000081487d8e0702','hex'),5067),
(0,decode('000081327d8f0704','hex'),5062),
(0,decode('0000813d7d8f0701','hex'),5067),
(0,decode('0000814b7d8f0701','hex'),5062),
(0,decode('0000814e7d8f0703','hex'),5061),
(0,decode('0000814f7d8f0703','hex'),5061),
(0,decode('000081327d900703','hex'),5062),
(0,decode('0000814a7d900702','hex'),5062),
(0,decode('0000814b7d900702','hex'),5061),
(0,decode('0000814c7d900704','hex'),5062),
(0,decode('0000814c7d900705','hex'),5063),
(0,decode('0000814e7d900702','hex'),5067),
(0,decode('0000814e7d900703','hex'),5066),
(0,decode('0000814f7d900702','hex'),5062),
(0,decode('000081507d900703','hex'),5061),
(0,decode('000081507d900704','hex'),5063),
(0,decode('000081497d910702','hex'),5062),
(0,decode('000081507d910703','hex'),5061),
(0,decode('000081517d910703','hex'),5062),
(0,decode('000081377d920701','hex'),5061),
(0,decode('000081387d920701','hex'),5061),
(0,decode('0000813a7d920701','hex'),5061),
(0,decode('0000813c7d920701','hex'),5061),
(0,decode('0000813f7d920702','hex'),5062),
(0,decode('000081407d920702','hex'),5061),
(0,decode('000081417d920702','hex'),5061),
(0,decode('000081427d920702','hex'),5065),
(0,decode('000081427d920704','hex'),5061),
(0,decode('000081447d920702','hex'),5061),
(0,decode('000081457d920702','hex'),5064),
(0,decode('000081457d920704','hex'),5062),
(0,decode('000081497d920701','hex'),5062),
(0,decode('0000814a7d920702','hex'),5067),
(0,decode('0000814a7d920703','hex'),5065),
(0,decode('0000814a7d920705','hex'),5061),
(0,decode('000081557d920701','hex'),5061),
(0,decode('000081587d920701','hex'),5061),
(0,decode('000081597d920701','hex'),5061),
(0,decode('0000815a7d920701','hex'),5061),
(0,decode('000081327d930703','hex'),5062),
(0,decode('000081367d930701','hex'),5062),
(0,decode('0000813a7d930703','hex'),5061),
(0,decode('0000813b7d930702','hex'),5061),
(0,decode('0000813c7d930703','hex'),5062),
(0,decode('0000813f7d930702','hex'),5062),
(0,decode('000081407d930703','hex'),5062),
(0,decode('000081447d930702','hex'),5062),
(0,decode('000081497d930701','hex'),5062),
(0,decode('0000814a7d930703','hex'),5061),
(0,decode('0000814a7d930704','hex'),5062),
(0,decode('000081507d930702','hex'),5061),
(0,decode('000081547d930701','hex'),5062),
(0,decode('000081567d930702','hex'),5061),
(0,decode('000081577d930703','hex'),5061),
(0,decode('000081327d940703','hex'),5062),
(0,decode('0000813f7d940701','hex'),5062),
(0,decode('000081407d940702','hex'),5065),
(0,decode('000081457d940702','hex'),5067),
(0,decode('000081497d940702','hex'),5062),
(0,decode('0000814a7d940703','hex'),5062),
(0,decode('0000814b7d940702','hex'),5062),
(0,decode('0000814f7d940702','hex'),5061),
(0,decode('000081517d940704','hex'),5063),
(0,decode('000081547d940701','hex'),5062),
(0,decode('000081557d940703','hex'),5062),
(0,decode('0000815a7d940702','hex'),5062),
(0,decode('000081327d950703','hex'),5062),
(0,decode('0000813f7d950701','hex'),5062),
(0,decode('000081407d950703','hex'),5062),
(0,decode('000081457d950703','hex'),5062),
(0,decode('0000814a7d950702','hex'),5062),
(0,decode('0000814b7d950703','hex'),5061),
(0,decode('0000814c7d950704','hex'),5061),
(0,decode('0000814c7d950705','hex'),5062),
(0,decode('000081507d950704','hex'),5063),
(0,decode('000081547d950702','hex'),5062),
(0,decode('000081557d950703','hex'),5062),
(0,decode('000081557d950704','hex'),5063),
(0,decode('0000815a7d950702','hex'),5063),
(0,decode('000081367d960701','hex'),5062),
(0,decode('000081377d960702','hex'),5062),
(0,decode('000081387d960701','hex'),5065),
(0,decode('0000813b7d960701','hex'),5063),
(0,decode('0000813b7d960702','hex'),5061),
(0,decode('0000813c7d960702','hex'),5062),
(0,decode('0000813f7d960701','hex'),5062),
(0,decode('000081407d960703','hex'),5062),
(0,decode('000081417d960702','hex'),5061),
(0,decode('000081457d960703','hex'),5062),
(0,decode('000081367d970702','hex'),5062),
(0,decode('000081377d970703','hex'),5061),
(0,decode('000081387d970704','hex'),5061),
(0,decode('000081387d970705','hex'),5062),
(0,decode('0000813b7d970702','hex'),5061),
(0,decode('0000813c7d970702','hex'),5063),
(0,decode('0000813f7d970701','hex'),5062),
(0,decode('000081407d970703','hex'),5062),
(0,decode('000081417d970704','hex'),5061),
(0,decode('000081417d970705','hex'),5063),
(0,decode('000081447d970701','hex'),5062),
(0,decode('000081457d970704','hex'),5063),
(0,decode('000081327d980702','hex'),5064),
(0,decode('000081327d980704','hex'),5062),
(0,decode('000081547d980702','hex'),5062),
(0,decode('000081557d980702','hex'),5062),
(0,decode('0000815a7d980703','hex'),5062),
(0,decode('000081327d990703','hex'),5062),
(0,decode('000081337d990701','hex'),5064),
(0,decode('000081337d990702','hex'),5061),
(0,decode('000081347d990701','hex'),5061),
(0,decode('000081357d990701','hex'),5061),
(0,decode('000081367d990701','hex'),5061),
(0,decode('000081377d990701','hex'),5061),
(0,decode('000081547d990701','hex'),5062),
(0,decode('000081577d990701','hex'),5061),
(0,decode('000081587d990701','hex'),5061),
(0,decode('000081597d990703','hex'),5061),
(0,decode('000081327d9a0702','hex'),5062),
(0,decode('000081337d9a0703','hex'),5061),
(0,decode('000081347d9a0702','hex'),5061),
(0,decode('000081367d9a0702','hex'),5061),
(0,decode('000081447d9a0701','hex'),5062),
(0,decode('000081467d9a0703','hex'),5061),
(0,decode('000081497d9a0702','hex'),5061),
(0,decode('0000814a7d9a0702','hex'),5065),
(0,decode('0000814a7d9a0703','hex'),5062),
(0,decode('000081547d9a0701','hex'),5062),
(0,decode('000081557d9a0703','hex'),5064),
(0,decode('000081557d9a0704','hex'),5061),
(0,decode('000081557d9a0705','hex'),5062),
(0,decode('000081567d9a0702','hex'),5064),
(0,decode('000081577d9a0703','hex'),5064),
(0,decode('000081577d9a0704','hex'),5061),
(0,decode('000081587d9a0703','hex'),5064),
(0,decode('0000815a7d9a0705','hex'),5063),
(0,decode('000081267d9b0702','hex'),5065),
(0,decode('000081327d9b0703','hex'),5062),
(0,decode('000081367d9b0701','hex'),5062),
(0,decode('000081377d9b0704','hex'),5063),
(0,decode('000081447d9b0701','hex'),5062),
(0,decode('000081497d9b0702','hex'),5062),
(0,decode('0000814a7d9b0703','hex'),5062),
(0,decode('000081237d9c0702','hex'),5065),
(0,decode('000081237d9c0703','hex'),5067),
(0,decode('000081257d9c0702','hex'),5063),
(0,decode('0000812a7d9c0702','hex'),5063),
(0,decode('000081447d9c0702','hex'),5062),
(0,decode('000081457d9c0703','hex'),5063),
(0,decode('000081217d9d0702','hex'),5067),
(0,decode('0000812c7d9d0702','hex'),5067),
(0,decode('000081327d9d0702','hex'),5064),
(0,decode('000081497d9d0702','hex'),5062),
(0,decode('0000814a7d9d0703','hex'),5062),
(0,decode('000081327d9e0703','hex'),5062),
(0,decode('000081337d9e0702','hex'),5061),
(0,decode('000081347d9e0702','hex'),5061),
(0,decode('000081357d9e0701','hex'),5061),
(0,decode('000081367d9e0701','hex'),5062),
(0,decode('000081367d9e0702','hex'),5061),
(0,decode('000081367d9e0703','hex'),5063),
(0,decode('000081377d9e0703','hex'),5062),
(0,decode('000081457d9e0701','hex'),5061),
(0,decode('0000814a7d9e0703','hex'),5062),
(0,decode('000081507d9e0703','hex'),5061),
(0,decode('000081527d9e0702','hex'),5061),
(0,decode('000081537d9e0703','hex'),5062),
(0,decode('000081217d9f0702','hex'),5067),
(0,decode('0000812b7d9f0701','hex'),5067),
(0,decode('000081327d9f0702','hex'),5067),
(0,decode('000081347d9f0702','hex'),5066),
(0,decode('000081357d9f0703','hex'),5061),
(0,decode('000081367d9f0703','hex'),5061),
(0,decode('000081447d9f0702','hex'),5062),
(0,decode('000081457d9f0703','hex'),5062),
(0,decode('000081497d9f0702','hex'),5062),
(0,decode('0000814f7d9f0704','hex'),5062),
(0,decode('0000814f7d9f0705','hex'),5061),
(0,decode('000081527d9f0702','hex'),5062),
(0,decode('000081537d9f0703','hex'),5062),
(0,decode('000081217da00703','hex'),5063),
(0,decode('0000812b7da00701','hex'),5062),
(0,decode('0000812d7da00702','hex'),5063),
(0,decode('000081447da00701','hex'),5062),
(0,decode('000081497da00702','hex'),5062),
(0,decode('000081497da00703','hex'),5063),
(0,decode('0000814a7da00703','hex'),5062),
(0,decode('0000812c7da10702','hex'),5065),
(0,decode('0000812d7da10702','hex'),5067),
(0,decode('000081447da10701','hex'),5062),
(0,decode('000081457da10703','hex'),5064),
(0,decode('000081457da10704','hex'),5061),
(0,decode('000081467da10703','hex'),5064),
(0,decode('000081467da10704','hex'),5061),
(0,decode('000081477da10702','hex'),5064),
(0,decode('000081487da10703','hex'),5064),
(0,decode('000081497da10702','hex'),5064),
(0,decode('000081497da10703','hex'),5061),
(0,decode('0000814a7da10704','hex'),5063),
(0,decode('000081207da20702','hex'),5066),
(0,decode('000081237da20702','hex'),5065),
(0,decode('000081257da20702','hex'),5063),
(0,decode('0000814f7da20702','hex'),5062),
(0,decode('000081507da20701','hex'),5061),
(0,decode('000081517da20701','hex'),5061),
(0,decode('000081527da20702','hex'),5062),
(0,decode('000081527da20703','hex'),5061),
(0,decode('000081297da30702','hex'),5067),
(0,decode('0000812d7da30703','hex'),5063),
(0,decode('000081487da30701','hex'),5065),
(0,decode('0000814f7da30703','hex'),5064),
(0,decode('0000814f7da30704','hex'),5061),
(0,decode('000081507da30703','hex'),5064),
(0,decode('000081517da30702','hex'),5065),
(0,decode('000081517da30703','hex'),5064),
(0,decode('000081527da30702','hex'),5064),
(0,decode('000081527da30703','hex'),5062),
(0,decode('000081537da30702','hex'),5065),
(0,decode('000081537da30705','hex'),5063),
(0,decode('000081237da40702','hex'),5064),
(0,decode('000081247da40702','hex'),5067),
(0,decode('000081287da40701','hex'),5064),
(0,decode('000081467da40701','hex'),5063),
(0,decode('000081487da40702','hex'),5062),
(0,decode('000081497da40702','hex'),5061),
(0,decode('000081497da40703','hex'),5062),
(0,decode('0000814a7da40703','hex'),5062),
(0,decode('000081297da50702','hex'),5063),
(0,decode('000081347da50702','hex'),5067),
(0,decode('000081467da50703','hex'),5061),
(0,decode('000081477da50705','hex'),5061),
(0,decode('0000814a7da50702','hex'),5061),
(0,decode('0000814b7da50703','hex'),5062),
(0,decode('000081237da60702','hex'),5065),
(0,decode('0000812f7da60701','hex'),5066),
(0,decode('000081327da60702','hex'),5065),
(0,decode('000081357da60703','hex'),5063),
(0,decode('0000814b7da60702','hex'),5061),
(0,decode('0000814b7da60703','hex'),5062),
(0,decode('000081367da70702','hex'),5067),
(0,decode('000081367da70703','hex'),5063),
(0,decode('000081457da70701','hex'),5064),
(0,decode('000081457da70702','hex'),5065),
(0,decode('0000814b7da70701','hex'),5062),
(0,decode('0000814c7da70702','hex'),5065),
(0,decode('0000814c7da70704','hex'),5062),
(0,decode('000081247da80703','hex'),5063),
(0,decode('000081307da80702','hex'),5065),
(0,decode('000081457da80702','hex'),5062),
(0,decode('0000814a7da80701','hex'),5064),
(0,decode('0000814b7da80702','hex'),5062),
(0,decode('0000812e7da90702','hex'),5063),
(0,decode('000081457da90703','hex'),5062),
(0,decode('000081457da90704','hex'),5061),
(0,decode('000081467da90702','hex'),5062),
(0,decode('0000814a7da90701','hex'),5062),
(0,decode('0000814b7da90702','hex'),5062),
(0,decode('0000812a7daa0703','hex'),5063),
(0,decode('000081467daa0703','hex'),5062),
(0,decode('000081467daa0704','hex'),5061),
(0,decode('000081477daa0702','hex'),5065),
(0,decode('000081477daa0703','hex'),5062),
(0,decode('000081497daa0701','hex'),5062),
(0,decode('0000814a7daa0702','hex'),5062),
(0,decode('000081397dab0703','hex'),5063),
(0,decode('000081467dab0701','hex'),5062),
(0,decode('000081477dab0704','hex'),5064),
(0,decode('000081477dab0705','hex'),5061),
(0,decode('000081477dab0706','hex'),5062),
(0,decode('000081487dab0703','hex'),5064),
(0,decode('000081487dab0704','hex'),5062),
(0,decode('000081497dab0702','hex'),5065),
(0,decode('000081497dab0704','hex'),5064),
(0,decode('0000814a7dab0704','hex'),5063),
(0,decode('000081307dad0702','hex'),5061),
(0,decode('000081257dae0701','hex'),5064),
(0,decode('000081307dae0702','hex'),5067),
(0,decode('000081317dae0702','hex'),5066),
(0,decode('000081377dae0702','hex'),5065),
(0,decode('000081367daf0703','hex'),5063),
(0,decode('000081317db00701','hex'),5067),
(0,decode('000081237db40702','hex'),5064),
(0,decode('000081247db40702','hex'),5065),
(0,decode('000081277db40704','hex'),5063),
(0,decode('0000813b7db50701','hex'),5067),
(0,decode('000081227db60703','hex'),5062),
(0,decode('000081227db70703','hex'),5062),
(0,decode('000081227db90702','hex'),5067),
(0,decode('0000812b7db90702','hex'),5062),
(0,decode('000081377db90702','hex'),5062),
(0,decode('000081247dba0702','hex'),5066),
(0,decode('0000812c7dba0702','hex'),5063),
(0,decode('000081377dba0702','hex'),5067),
(0,decode('000081307dbb0702','hex'),5067),
(0,decode('000081327dbb0703','hex'),5063),
(0,decode('000081377dbb0702','hex'),5067),
(0,decode('0000813b7dbb0701','hex'),5064),
(0,decode('0000811b7dbc0701','hex'),5066),
(0,decode('000081347dbc0702','hex'),5064),
(0,decode('000081357dbc0703','hex'),5063),
(0,decode('000081377dbc0702','hex'),5064),
(0,decode('0000813e7dbc0702','hex'),5061),
(0,decode('0000813f7dbc0702','hex'),5065),
(0,decode('000081407dbc0702','hex'),5063),
(0,decode('0000811d7dbd0702','hex'),5066),
(0,decode('0000813c7dbd0702','hex'),5065),
(0,decode('0000813d7dbd0702','hex'),5066),
(0,decode('0000811f7dbe0702','hex'),5063),
(0,decode('0000813d7dc00702','hex'),5063),
(0,decode('0000811b7dc10703','hex'),5063),
(0,decode('0000811e7dc10702','hex'),5066),
(0,decode('000081257dc10704','hex'),5064),
(0,decode('000081267dc10706','hex'),5062),
(0,decode('000081407dc10703','hex'),5063),
(0,decode('000081207dc20702','hex'),5065),
(0,decode('000081207dc50702','hex'),5067),
(0,decode('000081207dc50703','hex'),5063),
(0,decode('0000811d7dc60702','hex'),5063),
(0,decode('000081217dc60703','hex'),5067),
(0,decode('0000811d7dc70702','hex'),5063),
(0,decode('000081257dc80702','hex'),5063),
(0,decode('000081287dc80703','hex'),5063),
(0,decode('0000811d7dc90702','hex'),5066),
(0,decode('000081217dc90703','hex'),5063),
(0,decode('000081247dc90702','hex'),5066),
(0,decode('000081237dca0701','hex'),5067),
(0,decode('000081247dcb0702','hex'),5066),
(0,decode('000081257dcc0703','hex'),5064),
(0,decode('000081267dcc0702','hex'),5067),
(0,decode('000081637d950802','hex'),5066),
(0,decode('000081637d960802','hex'),5064),
(0,decode('000081637d960803','hex'),5067),
(0,decode('000081677d960802','hex'),5065),
(0,decode('000081687d960802','hex'),5064),
(0,decode('000081697d960802','hex'),5065),
(0,decode('000081667d970801','hex'),5067),
(0,decode('000081667d980801','hex'),5067),
(0,decode('000081407dc80802','hex'),5064),
(0,decode('000081417dc80802','hex'),5064),
(0,decode('000081417dc80803','hex'),5065),
(1,decode('000081717d920700','hex'),12721),
(1,decode('000081727d920700','hex'),12721),
(1,decode('000081737d920700','hex'),12721),
(1,decode('000081747d920700','hex'),12721),
(1,decode('000081757d920700','hex'),12721),
(1,decode('000081767d920700','hex'),12721),
(1,decode('000081777d920700','hex'),12721),
(1,decode('000081787d920700','hex'),12721),
(1,decode('000081717d930700','hex'),12721),
(1,decode('000081727d930700','hex'),12721),
(1,decode('000081737d930700','hex'),12721),
(1,decode('000081747d930700','hex'),12721),
(1,decode('000081757d930700','hex'),12721),
(1,decode('000081767d930700','hex'),12721),
(1,decode('000081777d930700','hex'),12721),
(1,decode('000081787d930700','hex'),12721),
(1,decode('000081717d940700','hex'),12721),
(1,decode('000081727d940700','hex'),12721),
(1,decode('000081737d940700','hex'),12721),
(1,decode('000081747d940700','hex'),12721),
(1,decode('000081757d940700','hex'),12721),
(1,decode('000081767d940700','hex'),12721),
(1,decode('000081777d940700','hex'),12721),
(1,decode('000081787d940700','hex'),12721),
(1,decode('000081717d950700','hex'),12721),
(1,decode('000081727d950700','hex'),12721),
(1,decode('000081737d950700','hex'),12721),
(1,decode('000081747d950700','hex'),12721),
(1,decode('000081757d950700','hex'),12721),
(1,decode('000081767d950700','hex'),12721),
(1,decode('000081777d950700','hex'),12721),
(1,decode('000081787d950700','hex'),12721),
(1,decode('000081717d960700','hex'),12721),
(1,decode('000081727d960700','hex'),12721),
(1,decode('000081737d960700','hex'),12721),
(1,decode('000081747d960700','hex'),12721),
(1,decode('000081757d960700','hex'),12721),
(1,decode('000081767d960700','hex'),12721),
(1,decode('000081777d960700','hex'),12721),
(1,decode('000081787d960700','hex'),12721),
(1,decode('000081717d970700','hex'),12721),
(1,decode('000081727d970700','hex'),12721),
(1,decode('000081737d970700','hex'),12721),
(1,decode('000081747d970700','hex'),12721),
(1,decode('000081757d970700','hex'),12721),
(1,decode('000081767d970700','hex'),12721),
(1,decode('000081777d970700','hex'),12721),
(1,decode('000081787d970700','hex'),12721),
(1,decode('000081717d980700','hex'),12721),
(1,decode('000081727d980700','hex'),12721),
(1,decode('000081737d980700','hex'),12721),
(1,decode('000081747d980700','hex'),12721),
(1,decode('000081757d980700','hex'),12721),
(1,decode('000081767d980700','hex'),12721),
(1,decode('000081777d980700','hex'),12721),
(1,decode('000081787d980700','hex'),12721)) source(variant,placement,appearance)
  WHERE source.variant=p_variant AND source.placement=substring(p_placement FROM 33 FOR 8)
    AND source.appearance=p_before)
$$;

CREATE FUNCTION game_thornfire_station_pair(c game_item_craft_records,r game_terrain_tool_receipts)
RETURNS BOOLEAN LANGUAGE SQL STABLE AS $$
 SELECT c.consume_source AND c.created_xact_id=pg_current_xact_id()
  AND r.created_xact_id=c.created_xact_id
  AND (c.world_id,c.character_id,c.game_session_id,c.command_id,c.intent,c.source_item_instance_id)
    =(r.world_id,r.character_id,r.game_session_id,r.command_id,r.intent,r.tool_item_instance_id)
  AND jsonb_array_length(c.outputs)=1 AND c.outputs->0->>'count'='1'
  AND EXISTS(SELECT 1 FROM game_world_reset_active a
    JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
    JOIN game_world_change_days d USING(world_id,reset_epoch)
    WHERE a.world_id=r.world_id AND a.reset_epoch=r.reset_epoch
      AND NOT EXISTS(SELECT 1 FROM game_world_reset_jobs j WHERE j.world_id=a.world_id AND j.phase='RETIRING')
      AND c.occurred_at>=floor(extract(epoch FROM e.activated_at)*1000)::bigint
      AND c.occurred_at<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint
      AND game_thornfire_source_station((d.decision->'variants'->0->>15)::int,r.placement,r.before_item_id)
      AND CASE c.recipe
       WHEN 'oteryn:world-change.contribution' THEN
        d.decision->'variants'->0->>15='0'
        AND c.before_item->>'definition_production_key'='oteryn:item.tibia.i12722'
        AND c.outputs->0->>'key'='oteryn:item.tibia.i12819'
        AND (c.outputs->0->'decay' IS NULL OR c.outputs->0->'decay'='null'::jsonb)
        AND r.transformed_item_id=0 AND r.before_item_id IN (232,5061,5062,5063,5064,5065,5066,5067,6288,6289,7803,19096,32021)
       WHEN 'oteryn:world-change.refill' THEN
        c.before_item->>'definition_production_key'='oteryn:item.tibia.i12819'
        AND r.before_item_id=12721 AND r.transformed_item_id=12721
        AND c.outputs->0->>'key'='oteryn:item.tibia.i12722'
        AND c.outputs->0->>'content_digest' ~ '^sha256:[0-9a-f]{64}$'
        AND c.outputs->0->'decay'=jsonb_build_object('duration_millis',3600000,'target',
          jsonb_build_object('family','Item','key','oteryn:item.tibia.i12819','revision','definition-r1'))
       ELSE FALSE END)
$$;

-- Source flame restoration belongs to the existing terrain row and its drainer.
-- The only possible deadlines are the next save-relative 8h or 16h boundary.
CREATE FUNCTION game_thornfire_station_decay(c game_item_craft_records,r game_terrain_tool_receipts,t game_terrain_tool_transforms)
RETURNS BOOLEAN LANGUAGE SQL STABLE AS $$
 SELECT (t.world_id,t.channel_id,t.placement,t.reset_epoch,t.state_item_id)
   =(r.world_id,r.channel_id,r.placement,r.reset_epoch,r.transformed_item_id)
  AND t.map_revision='sha256:'||encode(substring(r.placement FROM 1 FOR 32),'hex')
  AND t.decay_step=0
  AND CASE c.recipe WHEN 'oteryn:world-change.refill' THEN
    t.decay_states='{}'::bigint[] AND t.decay_delays='{}'::bigint[] AND t.decay_at_unix_ms IS NULL
   WHEN 'oteryn:world-change.contribution' THEN EXISTS(
    SELECT 1 FROM game_world_reset_epochs e
    CROSS JOIN LATERAL (SELECT min(floor(extract(epoch FROM e.activated_at)*1000)::bigint+offset_ms) deadline
      FROM unnest(ARRAY[28800000,57600000]::bigint[]) offset_ms
      WHERE floor(extract(epoch FROM e.activated_at)*1000)::bigint+offset_ms>c.occurred_at) next
    WHERE e.world_id=r.world_id AND e.reset_epoch=r.reset_epoch AND
     CASE WHEN next.deadline IS NULL THEN t.decay_states='{}'::bigint[]
       AND t.decay_delays='{}'::bigint[] AND t.decay_at_unix_ms IS NULL
      ELSE t.decay_states=ARRAY[r.before_item_id]::bigint[]
       AND t.decay_delays=ARRAY[28800000]::bigint[] AND t.decay_at_unix_ms=next.deadline END)
   ELSE FALSE END
$$;

-- Extend the existing reciprocal contribution predicate; its complete Awash arm survives.
DO $migration$
DECLARE body TEXT; before TEXT; after TEXT;
BEGIN
 body:=pg_get_functiondef('game_world_change_craft_pair(game_item_craft_records,game_terrain_tool_receipts,game_world_change_contributions)'::regprocedure);
 before:=$old$c.before_item->>'definition_production_key'='oteryn:item.tibia.i12722'$old$;
 after:='game_thornfire_station_pair(c,r) AND '||before;
 IF length(body)-length(replace(body,before,''))<>length(before) THEN
  RAISE EXCEPTION 'missing unique Thornfire contribution arm'; END IF;
 EXECUTE replace(body,before,after);
 body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
 before:=$old$AND t.state_item_id=r.transformed_item_id
            AND t.decay_states='{}'::bigint[] AND t.decay_delays='{}'::bigint[])$old$;
 after:=$new$AND t.state_item_id=r.transformed_item_id
            AND ((w.resource='awash_coal' AND t.decay_states='{}'::bigint[] AND t.decay_delays='{}'::bigint[])
             OR (w.resource='thornfire_extinguished' AND game_thornfire_station_decay(NEW,r,t))))$new$;
 IF length(body)-length(replace(body,before,''))<>length(before) THEN
  RAISE EXCEPTION 'missing unique station terrain guard'; END IF;
 EXECUTE replace(body,before,after);
END $migration$;

DO $migration$
DECLARE body TEXT; begin_at INT; arm TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_craft_admit()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing craft admission body'; END IF;
 arm:=$arm$
 IF NEW.recipe='oteryn:world-change.refill' THEN
  IF NOT EXISTS(SELECT 1 FROM game_terrain_tool_receipts r
    JOIN game_terrain_tool_transforms t ON (t.world_id,t.channel_id,t.placement,t.reset_epoch)=(r.world_id,r.channel_id,r.placement,r.reset_epoch)
    WHERE (r.game_session_id,r.command_id)=(NEW.game_session_id,NEW.command_id)
      AND game_thornfire_station_pair(NEW,r) AND game_thornfire_station_decay(NEW,r,t)
      AND NEW.outputs->0->>'content_digest'='sha256:'||encode(t.content_generation,'hex'))
  THEN RAISE EXCEPTION 'Thornfire refill requires exact same-XID source bog' USING ERRCODE='23514'; END IF;
 END IF;
 IF NEW.recipe='oteryn:world-change.contribution'
    AND NEW.before_item->>'definition_production_key'='oteryn:item.tibia.i12722' THEN
  IF NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p
    JOIN game_terrain_tool_receipts r ON (r.game_session_id,r.command_id)=(NEW.game_session_id,NEW.command_id)
    JOIN game_terrain_tool_transforms t ON (t.world_id,t.channel_id,t.placement,t.reset_epoch)=(r.world_id,r.channel_id,r.placement,r.reset_epoch)
    WHERE p.item_instance_id=NEW.source_item_instance_id AND p.expires_at_unix_ms>NEW.occurred_at
      AND p.expires_at_unix_ms>floor(extract(epoch FROM statement_timestamp())*1000)::bigint
      AND p.duration_millis=3600000 AND p.definition_family='Item'
      AND (p.definition_production_key,p.definition_revision)
       =(NEW.before_item->>'definition_production_key',NEW.before_item->>'definition_revision_ref')
      AND (p.target_family,p.target_production_key,p.target_revision)=('Item','oteryn:item.tibia.i12819','definition-r1')
      AND p.content_digest=t.content_generation
      AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts consumed WHERE consumed.item_instance_id=p.item_instance_id))
  THEN RAISE EXCEPTION 'Thornfire extinguish requires live original filled bucket' USING ERRCODE='23514'; END IF;
 END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;

-- A filled output must have exactly its existing-owner schedule. No craft-to-timer
-- admission is opened for other recipes, Items, durations or sources.
DO $migration$
DECLARE body TEXT; begin_at INT; arm TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF begin_at=0 THEN RAISE EXCEPTION 'missing Item timer source proof'; END IF;
 arm:=$arm$
 IF EXISTS(SELECT 1 FROM game_item_craft_records c
   JOIN game_terrain_tool_receipts r ON (r.game_session_id,r.command_id)=(c.game_session_id,c.command_id)
   JOIN game_terrain_tool_transforms t ON (t.world_id,t.channel_id,t.placement,t.reset_epoch)=(r.world_id,r.channel_id,r.placement,r.reset_epoch)
   JOIN game_item_instances i ON i.item_instance_id=NEW.item_instance_id
   WHERE c.transaction_id=NEW.source_transaction_id AND c.recipe='oteryn:world-change.refill'
    AND game_thornfire_station_pair(c,r) AND game_thornfire_station_decay(c,r,t)
    AND NEW.source_ordinal=1 AND NEW.created_xact_id=c.created_xact_id
    AND (c.outputs->0->>'item')::uuid=NEW.item_instance_id
    AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
      =(NEW.definition_family,NEW.definition_production_key,NEW.definition_revision)
    AND i.minted_transaction_id=c.transaction_id AND i.world_id=c.world_id AND i.quantity=1 AND i.lifecycle=1
    AND NEW.definition_family='Item' AND NEW.definition_production_key='oteryn:item.tibia.i12722'
    AND c.outputs->0->>'revision'=NEW.definition_revision
    AND NEW.duration_millis=3600000 AND NEW.expires_at_unix_ms=c.occurred_at+3600000
    AND (NEW.target_family,NEW.target_production_key,NEW.target_revision)=('Item','oteryn:item.tibia.i12819','definition-r1')
    AND NEW.content_digest=t.content_generation
    AND c.outputs->0->>'content_digest'='sha256:'||encode(NEW.content_digest,'hex')
    AND c.outputs->0->'decay'=jsonb_build_object('duration_millis',NEW.duration_millis,'target',
      jsonb_build_object('family',NEW.target_family,'key',NEW.target_production_key,'revision',NEW.target_revision)))
 THEN RETURN NULL; END IF;
$arm$;
 EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
END $migration$;

CREATE FUNCTION game_thornfire_refill_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p
   WHERE p.source_transaction_id=NEW.transaction_id AND p.source_ordinal=1
    AND p.item_instance_id=(NEW.outputs->0->>'item')::uuid AND p.created_xact_id=pg_current_xact_id())
 THEN RAISE EXCEPTION 'Thornfire refill requires its actual filled output timer' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_thornfire_refill_complete AFTER INSERT ON game_item_craft_records
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.recipe='oteryn:world-change.refill')
 EXECUTE FUNCTION game_thornfire_refill_complete();

CREATE FUNCTION game_thornfire_terrain_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_item_instances i WHERE i.item_instance_id=NEW.tool_item_instance_id
   AND i.definition_production_key IN('oteryn:item.tibia.i12722','oteryn:item.tibia.i12819'))
 AND NOT EXISTS(SELECT 1 FROM game_item_craft_records c
   JOIN game_terrain_tool_transforms t ON (t.world_id,t.channel_id,t.placement,t.reset_epoch)
    =(NEW.world_id,NEW.channel_id,NEW.placement,NEW.reset_epoch)
   WHERE (c.game_session_id,c.command_id)=(NEW.game_session_id,NEW.command_id)
     AND game_thornfire_station_pair(c,NEW) AND game_thornfire_station_decay(c,NEW,t))
 THEN RAISE EXCEPTION 'Thornfire terrain requires its atomic carried bucket debit' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_thornfire_terrain_complete AFTER INSERT ON game_terrain_tool_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_thornfire_terrain_complete();

DO $migration$
DECLARE f TEXT;
BEGIN
 FOREACH f IN ARRAY ARRAY['game_thornfire_station_pair(game_item_craft_records,game_terrain_tool_receipts)',
 'game_thornfire_station_decay(game_item_craft_records,game_terrain_tool_receipts,game_terrain_tool_transforms)',
 'game_thornfire_refill_complete()','game_thornfire_terrain_complete()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
END $migration$;
REVOKE ALL ON FUNCTION game_thornfire_source_station(INT,BYTEA,BIGINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_thornfire_source_station(INT,BYTEA,BIGINT),
 game_thornfire_station_pair(game_item_craft_records,game_terrain_tool_receipts),
 game_thornfire_station_decay(game_item_craft_records,game_terrain_tool_receipts,game_terrain_tool_transforms)
 TO oteryn_game_runtime;

-- Carrying the source-filled bucket retains its immutable absolute deadline.
-- Extend the existing action-source movement exception only for proved refill mints.
DO $migration$
DECLARE body TEXT; before TEXT; after TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_loop_line_admit()'::regprocedure);
 before:=$old$EXISTS (SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id = NEW.item_instance_id)$old$;
 after:=$new$(EXISTS (SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id = NEW.item_instance_id)
  AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules p
   JOIN game_item_craft_records c ON c.transaction_id=p.source_transaction_id
   WHERE p.item_instance_id=NEW.item_instance_id AND c.recipe='oteryn:world-change.refill'
    AND p.source_ordinal=1 AND (c.outputs->0->>'item')::uuid=p.item_instance_id
    AND p.definition_production_key='oteryn:item.tibia.i12722' AND p.duration_millis=3600000
    AND p.target_family='Item' AND p.target_production_key='oteryn:item.tibia.i12819'
    AND c.outputs->0->>'content_digest'='sha256:'||encode(p.content_digest,'hex')
    AND c.outputs->0->'decay'=jsonb_build_object('duration_millis',p.duration_millis,'target',
      jsonb_build_object('family',p.target_family,'key',p.target_production_key,'revision',p.target_revision))))$new$;
 IF length(body)-length(replace(body,before,''))<>length(before) THEN
  RAISE EXCEPTION 'missing unique absolute timer movement guard'; END IF;
 EXECUTE replace(body,before,after);
END $migration$;
