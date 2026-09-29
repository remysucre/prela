
WITH RankedPosts AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.Score,
        P.ViewCount,
        U.DisplayName AS OwnerDisplayName,
        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS Rank
    FROM 
        Posts P
    JOIN 
        Users U ON P.OwnerUserId = U.Id
    WHERE 
        P.CreationDate >= DATE '2023-01-01' 
        AND P.Score IS NOT NULL
),
UserStats AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        U.Reputation,
        U.UpVotes,
        U.DownVotes,
        U.Views,
        (U.UpVotes - U.DownVotes) AS VoteBalance
    FROM 
        Users U
    WHERE 
        U.Reputation >= 1000
),
PostSummary AS (
    SELECT 
        RP.PostId,
        RP.Title,
        RP.CreationDate,
        RP.Score,
        RP.ViewCount,
        RP.OwnerDisplayName,
        COUNT(CASE WHEN C.UserId IS NOT NULL THEN 1 END) AS CommentCount,
        COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpVoteCount,
        COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 3) AS DownVoteCount
    FROM 
        RankedPosts RP
    LEFT JOIN 
        Comments C ON RP.PostId = C.PostId
    LEFT JOIN 
        Votes V ON RP.PostId = V.PostId
    GROUP BY 
        RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName
)
SELECT 
    PS.PostId,
    PS.Title,
    PS.CreationDate,
    PS.Score,
    PS.ViewCount,
    PS.OwnerDisplayName,
    PS.CommentCount,
    U.VoteBalance,
    RP.Rank,
    CASE 
        WHEN PS.CommentCount > 10 THEN 'Hot'
        WHEN PS.Score > 50 THEN 'Popular'
        ELSE 'Normal'
    END AS PopularityRank
FROM 
    PostSummary PS
JOIN 
    UserStats U ON PS.OwnerDisplayName = U.DisplayName
JOIN 
    RankedPosts RP ON PS.PostId = RP.PostId
WHERE 
    PS.Score > 0
ORDER BY 
    PS.Score DESC,
    PS.ViewCount DESC, PS.PostId, U.UserId
LIMIT 50;
