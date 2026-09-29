SELECT 
    U.DisplayName AS UserDisplayName,
    P.Title AS PostTitle,
    P.CreationDate AS PostCreationDate,
    COUNT(C.ID) AS CommentCount
FROM 
    Posts P
JOIN 
    Users U ON P.OwnerUserId = U.Id
LEFT JOIN 
    Comments C ON P.Id = C.PostId
GROUP BY 
    U.DisplayName, P.Title, P.CreationDate
ORDER BY 
    CommentCount DESC, U.DisplayName, P.CreationDate, P.Title NULLS FIRST
LIMIT 10;
