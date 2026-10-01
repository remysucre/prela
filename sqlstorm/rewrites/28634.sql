
WITH RankedPosts AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.Body,
        P.CreationDate,
        U.DisplayName AS OwnerDisplayName,
        P.ViewCount,
        P.Score,
        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.ViewCount DESC, P.CreationDate DESC) AS rn
    FROM 
        Posts P
    JOIN 
        Users U ON P.OwnerUserId = U.Id
    WHERE 
        P.PostTypeId = 1   
        AND P.Score > 10   
),
PostTags AS (
    SELECT 
        P.Id AS PostId,
        STRING_AGG(TRIM(TAG.TagName), ', ' ORDER BY TRIM(TAG.TagName)) AS TagList
    FROM 
        Posts P
    CROSS JOIN LATERAL 
        (SELECT UNNEST(string_to_array(SUBSTRING(P.Tags, 2, LENGTH(P.Tags) - 2), '><')) AS TagName) AS TAG
    GROUP BY 
        P.Id
),
ClosedPosts AS (
    SELECT 
        PH.PostId, 
        PH.CreationDate AS ClosedDate, 
        C.Name AS CloseReason
    FROM 
        PostHistory PH
    JOIN 
        CloseReasonTypes C ON CAST(PH.Comment AS INT) = C.Id
    WHERE 
        PH.PostHistoryTypeId = 10  
)
SELECT 
    RP.PostId,
    RP.Title,
    RP.Body,
    RP.CreationDate AS QuestionDate,
    RP.OwnerDisplayName,
    RP.ViewCount,
    RP.Score,
    PT.TagList,
    CP.ClosedDate,
    CP.CloseReason
FROM 
    RankedPosts RP
LEFT JOIN 
    PostTags PT ON RP.PostId = PT.PostId
LEFT JOIN 
    ClosedPosts CP ON RP.PostId = CP.PostId
WHERE 
    RP.rn = 1   
ORDER BY 
    RP.Score DESC, RP.ViewCount DESC;
