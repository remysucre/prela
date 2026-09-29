WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        u.Reputation,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
    FROM 
        Posts p
    INNER JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') 
        AND u.Reputation > (SELECT AVG(Reputation) FROM Users) 
        AND COALESCE(p.ClosedDate, '9999-12-31') = '9999-12-31'
),
PostHistoryAggregates AS (
    SELECT 
        ph.PostId,
        COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
        COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount,
        COUNT(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 END) AS EditCount
    FROM 
        PostHistory ph
    GROUP BY 
        ph.PostId
),
PostVotes AS (
    SELECT 
        PostId,
        VoteTypeId,
        COUNT(*) AS VoteCount
    FROM 
        Votes 
    WHERE 
        VoteTypeId IN (2, 3) 
    GROUP BY 
        PostId, VoteTypeId
),
CombinedPostData AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.CreationDate,
        rp.Score,
        rp.ViewCount,
        rp.Reputation,
        COALESCE(pv2.VoteCount, 0) AS UpVotes,
        COALESCE(pv3.VoteCount, 0) AS DownVotes,
        COALESCE(ph.CloseCount, 0) AS CloseCount,
        COALESCE(ph.DeleteCount, 0) AS DeleteCount,
        COALESCE(ph.EditCount, 0) AS EditCount
    FROM 
        RankedPosts rp
    LEFT JOIN 
        PostVotes pv2 ON rp.PostId = pv2.PostId AND pv2.VoteTypeId = 2
    LEFT JOIN 
        PostVotes pv3 ON rp.PostId = pv3.PostId AND pv3.VoteTypeId = 3
    LEFT JOIN 
        PostHistoryAggregates ph ON rp.PostId = ph.PostId
)
SELECT 
    Title,
    CreationDate,
    Score,
    ViewCount,
    Reputation,
    UpVotes,
    DownVotes,
    CloseCount,
    DeleteCount,
    EditCount,
    CASE
        WHEN EditCount > 0 AND CloseCount = 0 THEN 'Active and Edited'
        WHEN CloseCount > 0 THEN 'Closed'
        WHEN DeleteCount > 0 THEN 'Deleted'
        ELSE 'Active'
    END AS PostStatus,
    CASE
        WHEN Reputation IS NULL THEN 'No Reputation'
        ELSE CONCAT('User Reputation: ', Reputation)
    END AS UserStatus
FROM 
    CombinedPostData
WHERE 
    (UpVotes - DownVotes) > 0
ORDER BY
    Score DESC,
    ViewCount DESC,
    PostId
LIMIT 100
OFFSET 0;