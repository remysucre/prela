
SELECT 
    P.Id AS PostId,
    P.Title,
    U.DisplayName AS OwnerDisplayName,
    P.CreationDate,
    P.ViewCount,
    P.Score,
    C.CommentCount,
    T.TagName
FROM 
    Posts P
JOIN 
    Users U ON P.OwnerUserId = U.Id
LEFT JOIN 
    (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
LEFT JOIN 
    (SELECT PostId, STRING_AGG(TagName, ', ' ORDER BY Id) AS TagName FROM Tags GROUP BY PostId) T ON P.Id = T.PostId
WHERE 
    P.PostTypeId = 1  
GROUP BY 
    P.Id, P.Title, U.DisplayName, P.CreationDate, P.ViewCount, P.Score, C.CommentCount, T.TagName
ORDER BY 
    P.CreationDate DESC
LIMIT 10;
