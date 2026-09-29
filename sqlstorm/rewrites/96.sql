WITH RankedPosts AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.Score,
        P.ViewCount,
        U.DisplayName AS OwnerDisplayName,
        ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY P.CreationDate DESC, P.Id) AS rn
    FROM 
        Posts P
    JOIN 
        Users U ON P.OwnerUserId = U.Id
    WHERE 
        P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' 
        AND P.Score > 0
),
TopUsers AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        COUNT(P.Id) AS PostCount,
        SUM(V.BountyAmount) AS TotalBounty
    FROM 
        Users U
    LEFT JOIN 
        Posts P ON U.Id = P.OwnerUserId
    LEFT JOIN 
        Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 
    GROUP BY 
        U.Id, U.DisplayName
    HAVING 
        COUNT(P.Id) > 10
),
ActiveUserPosts AS (
    SELECT 
        RP.PostId, 
        RP.Title, 
        RP.CreationDate, 
        RP.Score, 
        RP.ViewCount, 
        RP.OwnerDisplayName,
        TU.PostCount,
        TU.TotalBounty
    FROM 
        RankedPosts RP
    JOIN 
        TopUsers TU ON RP.OwnerDisplayName = TU.DisplayName
    WHERE 
        RP.rn = 1
)
SELECT 
    AUP.PostId,
    AUP.Title,
    AUP.CreationDate,
    AUP.Score,
    COALESCE(AUP.ViewCount, 0) AS ViewCount,
    AUP.OwnerDisplayName,
    AUP.PostCount,
    AUP.TotalBounty,
    CASE 
        WHEN AUP.TotalBounty > 0 THEN 'High Value' 
        ELSE 'Normal' 
    END AS ValueCategory 
FROM 
    ActiveUserPosts AUP
LEFT JOIN 
    Tags T ON AUP.Title ILIKE '%' || T.TagName || '%'
WHERE 
    T.TagName IS NOT NULL
ORDER BY 
    AUP.Score DESC, 
    AUP.ViewCount DESC, AUP.PostId, T.Id
LIMIT 100;