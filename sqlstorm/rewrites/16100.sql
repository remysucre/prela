SELECT 
    U.DisplayName AS UserDisplayName,
    P.Title AS PostTitle,
    P.CreationDate AS PostCreationDate,
    PH.CreationDate AS HistoryCreationDate,
    PHT.Name AS HistoryTypeName
FROM 
    Posts P
JOIN 
    PostHistory PH ON P.Id = PH.PostId
JOIN 
    PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
JOIN 
    Users U ON P.OwnerUserId = U.Id
WHERE 
    P.PostTypeId = 1 
ORDER BY 
    PH.CreationDate DESC, PH.Id
LIMIT 10;
