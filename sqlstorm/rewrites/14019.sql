WITH PostStats AS (
    SELECT 
        P.Id AS PostId,
        P.PostTypeId,
        U.Reputation AS OwnerReputation,
        P.CreationDate,
        P.LastActivityDate,
        P.ViewCount,
        P.Score,
        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
        COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount
    FROM 
        Posts P
    LEFT JOIN 
        Users U ON P.OwnerUserId = U.Id
    LEFT JOIN 
        Comments C ON P.Id = C.PostId
    LEFT JOIN 
        Posts A ON P.Id = A.ParentId
    GROUP BY 
        P.Id, P.PostTypeId, U.Reputation, P.CreationDate, P.LastActivityDate, P.ViewCount, P.Score
),
PostHistoryStats AS (
    SELECT 
        PH.PostId,
        MAX(PH.CreationDate) AS LastEditedDate,
        COUNT(PH.Id) AS EditCount
    FROM 
        PostHistory PH
    GROUP BY 
        PH.PostId
)
SELECT 
    PS.PostId,
    PS.PostTypeId,
    PS.OwnerReputation,
    PS.CreationDate,
    PS.LastActivityDate,
    PS.ViewCount,
    PS.Score,
    PS.CommentCount,
    PS.AnswerCount,
    PHS.LastEditedDate,
    PHS.EditCount
FROM 
    PostStats PS
LEFT JOIN 
    PostHistoryStats PHS ON PS.PostId = PHS.PostId
ORDER BY 
    PS.LastActivityDate DESC, PS.PostId
LIMIT 100;