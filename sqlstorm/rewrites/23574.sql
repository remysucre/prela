
WITH RankedPosts AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.Score,
        P.ViewCount,
        P.CreationDate,
        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.Id, V.Id) AS PostRank,
        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY P.Id) AS UpVotesCount,
        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY P.Id) AS DownVotesCount
    FROM 
        Posts P
    LEFT JOIN 
        Votes V ON P.Id = V.PostId
    WHERE 
        P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
),
ClosedPosts AS (
    SELECT 
        PH.PostId,
        PH.CreationDate,
        PH.Comment,
        C.Name AS CloseReason
    FROM 
        PostHistory PH
    JOIN 
        CloseReasonTypes C ON CAST(PH.Comment AS INTEGER) = C.Id
    WHERE 
        PH.PostHistoryTypeId IN (10, 11)
),
PostTags AS (
    SELECT 
        P.Id AS PostId,
        ARRAY_AGG(T.TagName ORDER BY T.TagName) AS Tags
    FROM 
        Posts P
    LEFT JOIN 
        LATERAL UNNEST(STRING_TO_ARRAY(P.Tags, '>')) AS T(TagName) ON TRUE
    GROUP BY 
        P.Id
),
UserBadges AS (
    SELECT 
        U.Id AS UserId,
        COUNT(B.Id) AS BadgeCount,
        STRING_AGG(B.Name, ', ' ORDER BY B.Name) AS BadgeNames
    FROM 
        Users U
    LEFT JOIN 
        Badges B ON U.Id = B.UserId
    GROUP BY 
        U.Id
)
SELECT 
    RP.Title,
    RP.Score,
    RP.ViewCount,
    RP.CreationDate,
    RP.UpVotesCount,
    RP.DownVotesCount,
    COALESCE(CP.CloseReason, 'Not Closed') AS CloseReason,
    PT.Tags,
    UB.BadgeCount,
    UB.BadgeNames
FROM 
    RankedPosts RP
LEFT JOIN 
    ClosedPosts CP ON RP.PostId = CP.PostId
LEFT JOIN 
    PostTags PT ON RP.PostId = PT.PostId
LEFT JOIN 
    UserBadges UB ON RP.PostId = UB.UserId
WHERE 
    RP.PostRank <= 5
    AND (RP.UpVotesCount - RP.DownVotesCount) > 0
ORDER BY 
    RP.Score DESC, 
    RP.ViewCount DESC;
